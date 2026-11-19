//! The restore system: every change Thomsen Optimizer makes to Windows
//! settings is written to a local journal **before** it is applied, together
//! with the value it replaces. That makes each change individually
//! reversible, lets whole batches roll back automatically when one step
//! fails, and lets the app undo leftovers of an interrupted game session on
//! the next start.
//!
//! Things that cannot be undone (processes that were closed, files that were
//! deleted) are recorded too, but are marked `irreversible` and never offered
//! for restore - we do not claim a recovery mechanism that does not exist.

use crate::error::{Error, Result};
use crate::store::DataDir;
use crate::sys::{Hive, PlanKind, RegData, SystemOps};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

const FILE: &str = "journal.json";
const MAX_RECORDS: usize = 600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChangeKind {
    #[serde(rename_all = "camelCase")]
    Registry { hive: Hive, path: String, name: String, original: Option<RegData>, new: Option<RegData> },
    #[serde(rename_all = "camelCase")]
    PowerPlan { original_guid: String, original_name: String, new_guid: String, new_name: String },
    #[serde(rename_all = "camelCase")]
    Irreversible { category: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeStatus {
    Pending,
    Applied,
    Restored,
    RestoreFailed,
    Irreversible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRecord {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub label: String,
    pub detail: String,
    /// `manual`, `session:<id>` (game booster) or `cleanup` / `process`.
    pub scope: String,
    pub kind: ChangeKind,
    pub status: ChangeStatus,
    pub restored_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl ChangeRecord {
    pub fn reversible(&self) -> bool {
        !matches!(self.kind, ChangeKind::Irreversible { .. })
    }
}

/// What the caller wants changed. The *original* value is read by the journal
/// itself, immediately before applying.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PlannedChange {
    #[serde(rename_all = "camelCase")]
    Registry { hive: Hive, path: String, name: String, new: Option<RegData>, label: String, detail: String },
    /// Switch the active power plan. `kind` is resolved against the plans
    /// that actually exist on this PC.
    #[serde(rename_all = "camelCase")]
    PowerPlan { target: PlanKind, label: String, detail: String },
}

impl PlannedChange {
    pub fn label(&self) -> &str {
        match self {
            PlannedChange::Registry { label, .. } | PlannedChange::PowerPlan { label, .. } => label,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Applied,
    AlreadySet,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub label: String,
    pub outcome: Outcome,
    pub message: Option<String>,
    pub record_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub restored: usize,
    pub failed: Vec<String>,
    pub nothing_to_restore: bool,
}

pub struct Journal {
    data: DataDir,
    ops: Arc<dyn SystemOps>,
    records: Mutex<Vec<ChangeRecord>>,
}

impl Journal {
    pub fn new(data: DataDir, ops: Arc<dyn SystemOps>) -> Self {
        let records: Vec<ChangeRecord> = data.read_json(FILE);
        Self { data, ops, records: Mutex::new(records) }
    }

    pub fn list(&self) -> Vec<ChangeRecord> {
        let mut v = self.records.lock().unwrap().clone();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    pub fn active_count(&self) -> usize {
        self.records.lock().unwrap().iter().filter(|r| r.reversible() && matches!(r.status, ChangeStatus::Applied | ChangeStatus::Pending | ChangeStatus::RestoreFailed)).count()
    }

    fn persist(&self, recs: &mut Vec<ChangeRecord>) {
        if recs.len() > MAX_RECORDS {
            // Only completed history is ever dropped; live changes are kept.
            let mut excess = recs.len() - MAX_RECORDS;
            recs.retain(|r| {
                if excess > 0 && matches!(r.status, ChangeStatus::Restored | ChangeStatus::Irreversible) {
                    excess -= 1;
                    false
                } else {
                    true
                }
            });
        }
        if let Err(e) = self.data.write_json(FILE, &*recs) {
            log::error!("could not write the restore journal: {e}");
        }
    }

    /// Records something that already happened and cannot be undone.
    pub fn log_irreversible(&self, scope: &str, category: &str, label: &str, detail: &str) {
        let mut recs = self.records.lock().unwrap();
        recs.push(ChangeRecord {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            label: label.into(),
            detail: detail.into(),
            scope: scope.into(),
            kind: ChangeKind::Irreversible { category: category.into() },
            status: ChangeStatus::Irreversible,
            restored_at: None,
            error: None,
        });
        self.persist(&mut recs);
    }

    fn resolve_plan(&self, target: PlanKind) -> Result<(String, String)> {
        let plans = self.ops.power_plans()?;
        let found = match target {
            PlanKind::Ultimate => {
                let p = self.ops.power_ensure_ultimate()?;
                Some((p.guid, p.name))
            }
            k => plans.iter().find(|p| p.kind == k).map(|p| (p.guid.clone(), p.name.clone())),
        };
        found.ok_or_else(|| Error::NotFound("That power plan does not exist on this PC.".into()))
    }

    /// Applies `changes` as one transaction under `scope`. If any change
    /// fails, the ones already applied in this call are rolled back and the
    /// error is returned together with the per-change outcomes.
    pub fn apply(&self, scope: &str, changes: &[PlannedChange]) -> std::result::Result<Vec<ApplyOutcome>, (Error, Vec<ApplyOutcome>)> {
        let mut outcomes: Vec<ApplyOutcome> = Vec::new();
        let mut applied_ids: Vec<String> = Vec::new();

        for ch in changes {
            match self.apply_one(scope, ch) {
                Ok(o) => {
                    if let (Outcome::Applied, Some(id)) = (o.outcome, &o.record_id) {
                        applied_ids.push(id.clone());
                    }
                    outcomes.push(o);
                }
                Err(e) => {
                    outcomes.push(ApplyOutcome { label: ch.label().to_string(), outcome: Outcome::Failed, message: Some(e.to_string()), record_id: None });
                    // Automatic rollback of this transaction.
                    for id in applied_ids.iter().rev() {
                        if let Err(re) = self.revert(id) {
                            log::error!("rollback of {id} failed: {re}");
                        }
                    }
                    for o in outcomes.iter_mut().filter(|o| o.outcome == Outcome::Applied) {
                        o.outcome = Outcome::Failed;
                        o.message = Some("Rolled back because a later step failed.".into());
                    }
                    return Err((e, outcomes));
                }
            }
        }
        Ok(outcomes)
    }

    fn apply_one(&self, scope: &str, ch: &PlannedChange) -> Result<ApplyOutcome> {
        let (kind, label, detail, is_noop) = match ch {
            PlannedChange::Registry { hive, path, name, new, label, detail } => {
                let original = self.ops.reg_get(*hive, path, name)?;
                let noop = original == *new;
                (ChangeKind::Registry { hive: *hive, path: path.clone(), name: name.clone(), original, new: new.clone() }, label, detail, noop)
            }
            PlannedChange::PowerPlan { target, label, detail } => {
                let (new_guid, new_name) = self.resolve_plan(*target)?;
                let current = self.ops.power_plans()?.into_iter().find(|p| p.active).ok_or_else(|| Error::NotFound("No active power plan found.".into()))?;
                let noop = current.guid.eq_ignore_ascii_case(&new_guid);
                (ChangeKind::PowerPlan { original_guid: current.guid, original_name: current.name, new_guid, new_name }, label, detail, noop)
            }
        };

        if is_noop {
            return Ok(ApplyOutcome { label: label.clone(), outcome: Outcome::AlreadySet, message: Some("Already set; nothing changed.".into()), record_id: None });
        }

        let id = uuid::Uuid::new_v4().to_string();
        // Write-ahead: the original value is on disk before we touch Windows.
        {
            let mut recs = self.records.lock().unwrap();
            recs.push(ChangeRecord { id: id.clone(), created_at: Utc::now(), label: label.clone(), detail: detail.clone(), scope: scope.into(), kind: kind.clone(), status: ChangeStatus::Pending, restored_at: None, error: None });
            self.persist(&mut recs);
        }

        let result = match &kind {
            ChangeKind::Registry { hive, path, name, new, .. } => match new {
                Some(d) => self.ops.reg_set(*hive, path, name, d),
                None => self.ops.reg_delete_value(*hive, path, name),
            },
            ChangeKind::PowerPlan { new_guid, .. } => self.ops.power_set_active(new_guid),
            ChangeKind::Irreversible { .. } => Ok(()),
        };

        let mut recs = self.records.lock().unwrap();
        if let Some(r) = recs.iter_mut().find(|r| r.id == id) {
            match &result {
                Ok(()) => r.status = ChangeStatus::Applied,
                Err(e) => {
                    // Never applied, so there is nothing to keep.
                    r.status = ChangeStatus::Restored;
                    r.restored_at = Some(Utc::now());
                    r.error = Some(e.to_string());
                }
            }
        }
        if result.is_err() {
            recs.retain(|r| r.id != id);
        }
        self.persist(&mut recs);
        drop(recs);

        result?;
        Ok(ApplyOutcome { label: label.clone(), outcome: Outcome::Applied, message: None, record_id: Some(id) })
    }

    fn undo(&self, kind: &ChangeKind) -> Result<()> {
        match kind {
            ChangeKind::Registry { hive, path, name, original, .. } => match original {
                Some(d) => self.ops.reg_set(*hive, path, name, d),
                None => self.ops.reg_delete_value(*hive, path, name),
            },
            ChangeKind::PowerPlan { original_guid, .. } => {
                if self.ops.power_plans()?.iter().any(|p| p.guid.eq_ignore_ascii_case(original_guid)) {
                    self.ops.power_set_active(original_guid)
                } else {
                    Err(Error::NotFound("The original power plan no longer exists.".into()))
                }
            }
            ChangeKind::Irreversible { .. } => Err(Error::Unsupported("This action cannot be undone.".into())),
        }
    }

    /// Restores a single change.
    pub fn revert(&self, id: &str) -> Result<()> {
        let kind = {
            let recs = self.records.lock().unwrap();
            let r = recs.iter().find(|r| r.id == id).ok_or_else(|| Error::NotFound("That change is not in the history.".into()))?;
            match r.status {
                ChangeStatus::Applied | ChangeStatus::Pending | ChangeStatus::RestoreFailed => {}
                ChangeStatus::Restored => return Err(Error::Invalid("That change was already restored.".into())),
                ChangeStatus::Irreversible => return Err(Error::Unsupported("This action cannot be undone.".into())),
            }
            r.kind.clone()
        };
        let res = self.undo(&kind);
        let mut recs = self.records.lock().unwrap();
        if let Some(r) = recs.iter_mut().find(|r| r.id == id) {
            match &res {
                Ok(()) => {
                    r.status = ChangeStatus::Restored;
                    r.restored_at = Some(Utc::now());
                    r.error = None;
                }
                Err(e) => {
                    r.status = ChangeStatus::RestoreFailed;
                    r.error = Some(e.to_string());
                }
            }
        }
        self.persist(&mut recs);
        res
    }

    fn revert_where(&self, pred: impl Fn(&ChangeRecord) -> bool) -> RestoreReport {
        // Newest first, so layered changes unwind in the right order.
        let mut targets: Vec<ChangeRecord> = self.records.lock().unwrap().iter().filter(|r| r.reversible() && matches!(r.status, ChangeStatus::Applied | ChangeStatus::Pending | ChangeStatus::RestoreFailed) && pred(r)).cloned().collect();
        targets.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let mut rep = RestoreReport { nothing_to_restore: targets.is_empty(), ..Default::default() };
        for t in targets {
            match self.revert(&t.id) {
                Ok(()) => rep.restored += 1,
                Err(e) => rep.failed.push(format!("{}: {e}", t.label)),
            }
        }
        rep
    }

    pub fn revert_scope(&self, scope: &str) -> RestoreReport {
        self.revert_where(|r| r.scope == scope)
    }

    pub fn revert_all(&self) -> RestoreReport {
        self.revert_where(|_| true)
    }

    /// Called at startup: undoes changes left behind by a game-booster
    /// session that never got to restore (crash, power loss, killed app).
    pub fn recover_after_crash(&self, live_scopes: &HashSet<String>) -> RestoreReport {
        self.revert_where(|r| r.scope.starts_with("session:") && !live_scopes.contains(&r.scope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::fake::FakeOps;
    use crate::sys::{GUID_HIGH, GUID_BALANCED};

    const P: &str = "Software\\Microsoft\\GameBar";

    fn setup() -> (tempfile::TempDir, Arc<FakeOps>, Journal) {
        let d = tempfile::tempdir().unwrap();
        let ops = Arc::new(FakeOps::with_default_plans());
        let j = Journal::new(DataDir::new(d.path()).unwrap(), ops.clone());
        (d, ops, j)
    }

    fn reg(name: &str, v: u32) -> PlannedChange {
        PlannedChange::Registry { hive: Hive::CurrentUser, path: P.into(), name: name.into(), new: Some(RegData::Dword(v)), label: name.into(), detail: String::new() }
    }

    #[test]
    fn apply_and_revert_registry_with_missing_original() {
        let (_d, ops, j) = setup();
        let out = j.apply("manual", &[reg("AutoGameModeEnabled", 1)]).unwrap();
        assert_eq!(out[0].outcome, Outcome::Applied);
        assert_eq!(ops.reg_get(Hive::CurrentUser, P, "AutoGameModeEnabled").unwrap(), Some(RegData::Dword(1)));
        j.revert(out[0].record_id.as_ref().unwrap()).unwrap();
        // The value did not exist before, so restoring deletes it again.
        assert_eq!(ops.reg_get(Hive::CurrentUser, P, "AutoGameModeEnabled").unwrap(), None);
        assert_eq!(j.active_count(), 0);
    }

    #[test]
    fn no_op_changes_are_not_recorded() {
        let (_d, ops, j) = setup();
        ops.reg_set(Hive::CurrentUser, P, "X", &RegData::Dword(1)).unwrap();
        let out = j.apply("manual", &[reg("X", 1)]).unwrap();
        assert_eq!(out[0].outcome, Outcome::AlreadySet);
        assert!(j.list().is_empty());
    }

    #[test]
    fn failed_batch_rolls_back_automatically() {
        let (_d, ops, j) = setup();
        ops.reg_set(Hive::CurrentUser, P, "A", &RegData::Dword(0)).unwrap();
        ops.fail_writes_to("B");
        let err = j.apply("manual", &[reg("A", 1), PlannedChange::PowerPlan { target: PlanKind::HighPerformance, label: "plan".into(), detail: String::new() }, reg("B", 1)]).unwrap_err();
        assert!(matches!(err.0, Error::PermissionDenied(_)));
        assert_eq!(ops.reg_get(Hive::CurrentUser, P, "A").unwrap(), Some(RegData::Dword(0)));
        assert_eq!(ops.active_guid().unwrap(), GUID_BALANCED);
        assert!(err.1.iter().all(|o| o.outcome == Outcome::Failed));
        assert_eq!(j.active_count(), 0);
    }

    #[test]
    fn power_plan_roundtrip_and_scope_restore() {
        let (_d, ops, j) = setup();
        j.apply("session:1", &[PlannedChange::PowerPlan { target: PlanKind::HighPerformance, label: "plan".into(), detail: String::new() }, reg("Z", 1)]).unwrap();
        assert_eq!(ops.active_guid().unwrap(), GUID_HIGH);
        let rep = j.revert_scope("session:1");
        assert_eq!(rep.restored, 2);
        assert_eq!(ops.active_guid().unwrap(), GUID_BALANCED);
        assert!(j.revert_scope("session:1").nothing_to_restore);
    }

    #[test]
    fn crash_recovery_only_touches_dead_sessions() {
        let (_d, ops, j) = setup();
        j.apply("session:dead", &[reg("D", 1)]).unwrap();
        j.apply("session:live", &[reg("L", 1)]).unwrap();
        j.apply("manual", &[reg("M", 1)]).unwrap();
        let live: HashSet<String> = ["session:live".to_string()].into();
        let rep = j.recover_after_crash(&live);
        assert_eq!(rep.restored, 1);
        assert_eq!(ops.reg_get(Hive::CurrentUser, P, "D").unwrap(), None);
        assert!(ops.reg_get(Hive::CurrentUser, P, "L").unwrap().is_some());
        assert!(ops.reg_get(Hive::CurrentUser, P, "M").unwrap().is_some());
    }

    #[test]
    fn irreversible_entries_are_never_restorable() {
        let (_d, _ops, j) = setup();
        j.log_irreversible("cleanup", "files", "Deleted 3 files", "1 MB");
        let id = j.list()[0].id.clone();
        assert!(j.revert(&id).is_err());
        assert!(j.revert_all().nothing_to_restore);
        assert_eq!(j.active_count(), 0);
    }

    #[test]
    fn journal_survives_restart() {
        let (d, ops, j) = setup();
        j.apply("manual", &[reg("P", 1)]).unwrap();
        let j2 = Journal::new(DataDir::new(d.path()).unwrap(), ops.clone());
        assert_eq!(j2.active_count(), 1);
        assert_eq!(j2.revert_all().restored, 1);
        assert_eq!(ops.reg_get(Hive::CurrentUser, P, "P").unwrap(), None);
    }
}
