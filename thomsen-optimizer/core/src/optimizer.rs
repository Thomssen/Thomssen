//! Turns analysis results and gaming profiles into concrete, journalled
//! changes, and records what actually happened. Every number in an
//! [`OptimizationResult`] is the count of an operation that really completed.

use crate::analysis::{self, HeavyApp, RecAction, Recommendation, Snapshot, GAMECFG, GAMEDVR};
use crate::error::{Error, Result};
use crate::journal::{ApplyOutcome, Journal, Outcome, PlannedChange};
use crate::procs::{self, ProcInfo};
use crate::profiles::{PowerChoice, ProfileConfig};
use crate::store::DataDir;
use crate::sys::{win, Hive, PlanKind, RegData, SystemOps};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::System;

const RESULTS_FILE: &str = "optimizations.json";
const MAX_RESULTS: usize = 30;

pub fn gather_snapshot(ops: &dyn SystemOps, sys: &System, procs_list: &[ProcInfo], cpu_avg: f32, startup_enabled: usize, temp_bytes: u64, gpu_names: Vec<String>, system_drive: Option<(u64, u64, String)>) -> Snapshot {
    let plans = ops.power_plans().unwrap_or_default();
    let dword = |hive, path: &str, name: &str| ops.reg_get(hive, path, name).ok().flatten().and_then(|d| d.as_dword());
    let mut heavy: Vec<HeavyApp> = procs::group(procs_list).into_iter().filter(|g| !g.protected).map(|g| HeavyApp { name: g.name, memory_bytes: g.memory_bytes, cpu: g.cpu }).collect();
    heavy.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    heavy.truncate(8);
    Snapshot {
        cpu_usage_avg: cpu_avg,
        logical_cores: sys.cpus().len(),
        mem_total: sys.total_memory(),
        mem_available: sys.available_memory(),
        has_high_perf_plan: plans.iter().any(|p| p.kind == PlanKind::HighPerformance),
        power_plan: plans.into_iter().find(|p| p.active),
        game_mode: dword(Hive::CurrentUser, analysis::GAMEBAR, "AutoGameModeEnabled"),
        app_capture: dword(Hive::CurrentUser, GAMEDVR, "AppCaptureEnabled"),
        historical_capture: dword(Hive::CurrentUser, GAMEDVR, "HistoricalCaptureEnabled"),
        game_dvr_enabled: dword(Hive::CurrentUser, GAMECFG, "GameDVR_Enabled"),
        heavy_apps: heavy,
        startup_enabled,
        temp_bytes,
        system_drive,
        gpu_names,
        uptime_secs: System::uptime(),
        on_battery: win::on_battery(),
    }
}

/// The settings a profile would change on *this* PC right now - skipping
/// anything already in the desired state, so we never record no-op changes.
pub fn profile_plan(ops: &dyn SystemOps, profile: &ProfileConfig) -> Result<Vec<PlannedChange>> {
    let mut v = Vec::new();
    let target = match profile.power {
        PowerChoice::Keep => None,
        PowerChoice::Balanced => Some(PlanKind::Balanced),
        PowerChoice::HighPerformance => Some(PlanKind::HighPerformance),
        PowerChoice::Ultimate => Some(PlanKind::Ultimate),
    };
    if let Some(mut kind) = target {
        if kind == PlanKind::Ultimate && ops.power_ensure_ultimate().is_err() {
            kind = PlanKind::HighPerformance; // not available on this PC
        }
        let label = match kind {
            PlanKind::Ultimate => "Power plan: Ultimate Performance",
            PlanKind::HighPerformance => "Power plan: High performance",
            PlanKind::Balanced => "Power plan: Balanced",
            _ => "Power plan",
        };
        v.push(PlannedChange::PowerPlan { target: kind, label: label.into(), detail: format!("Set by profile \"{}\"", profile.name) });
    }
    let get = |path: &str, name: &str| ops.reg_get(Hive::CurrentUser, path, name).ok().flatten().and_then(|d| d.as_dword());
    if profile.game_mode && get(analysis::GAMEBAR, "AutoGameModeEnabled") == Some(0) {
        v.push(analysis::reg_dword(analysis::GAMEBAR, "AutoGameModeEnabled", 1, "Windows Game Mode: on", &format!("Set by profile \"{}\"", profile.name)));
    }
    if profile.disable_game_dvr {
        let detail = format!("Set by profile \"{}\"", profile.name);
        for (path, name, label) in [
            (GAMEDVR, "AppCaptureEnabled", "Game Bar capture: off"),
            (GAMEDVR, "HistoricalCaptureEnabled", "Game Bar background recording: off"),
        ] {
            if get(path, name).is_some_and(|v| v != 0) {
                v.push(analysis::reg_dword(path, name, 0, label, &detail));
            }
        }
    }
    Ok(v)
}

/// Windows' per-app graphics preference (Settings > System > Display >
/// Graphics). `high == true` -> "High performance".
pub fn gpu_pref_change(exe: &str, high: bool) -> Result<PlannedChange> {
    let p = std::path::Path::new(exe);
    let ok = p.is_absolute() && exe.to_ascii_lowercase().ends_with(".exe") && !exe.chars().any(|c| c.is_control()) && exe.len() < 400;
    if !ok {
        return Err(Error::Invalid("Choose the full path of a game's .exe file.".into()));
    }
    Ok(PlannedChange::Registry {
        hive: Hive::CurrentUser,
        path: "Software\\Microsoft\\DirectX\\UserGpuPreferences".into(),
        name: exe.into(),
        new: Some(RegData::Sz(if high { "GpuPreference=2;".into() } else { "GpuPreference=0;".into() })),
        label: format!("Graphics preference for {}: {}", p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(), if high { "High performance" } else { "Let Windows decide" }),
        detail: "HKCU\\Software\\Microsoft\\DirectX\\UserGpuPreferences".into(),
    })
}

// -- Results -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultItem {
    /// `setting`, `profile`, `process`, `cleanup`, `info`
    pub kind: String,
    pub label: String,
    pub ok: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationResult {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub items: Vec<ResultItem>,
    pub applied: usize,
    pub already_set: usize,
    pub failed: usize,
    pub bytes_freed: u64,
}

impl OptimizationResult {
    pub fn new(title: &str) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), timestamp: Utc::now(), title: title.into(), items: vec![], applied: 0, already_set: 0, failed: 0, bytes_freed: 0 }
    }

    pub fn add_outcomes(&mut self, kind: &str, outcomes: &[ApplyOutcome]) {
        for o in outcomes {
            match o.outcome {
                Outcome::Applied => self.applied += 1,
                Outcome::AlreadySet => self.already_set += 1,
                Outcome::Failed => self.failed += 1,
            }
            if o.outcome != Outcome::AlreadySet {
                self.items.push(ResultItem { kind: kind.into(), label: o.label.clone(), ok: o.outcome == Outcome::Applied, detail: o.message.clone() });
            }
        }
    }

    pub fn add(&mut self, kind: &str, label: String, ok: bool, detail: Option<String>) {
        if ok {
            self.applied += 1;
        } else {
            self.failed += 1;
        }
        self.items.push(ResultItem { kind: kind.into(), label, ok, detail });
    }
}

pub fn save_result(data: &DataDir, r: &OptimizationResult) {
    let mut all: Vec<OptimizationResult> = data.read_json(RESULTS_FILE);
    all.insert(0, r.clone());
    all.truncate(MAX_RESULTS);
    if let Err(e) = data.write_json(RESULTS_FILE, &all) {
        log::warn!("could not save optimization result: {e}");
    }
}

pub fn load_results(data: &DataDir) -> Vec<OptimizationResult> {
    data.read_json(RESULTS_FILE)
}

/// Applies the settings of the recommendations named in `ids` (or, when
/// `ids` is `None`, every `safe_auto` one) as ONE transaction.
pub fn apply_recommendations(journal: &Journal, recs: &[Recommendation], ids: Option<&[String]>, title: &str) -> OptimizationResult {
    let mut result = OptimizationResult::new(title);
    let mut changes: Vec<PlannedChange> = Vec::new();
    for r in recs {
        let selected = match ids {
            Some(ids) => ids.contains(&r.id),
            None => r.safe_auto,
        };
        if !selected {
            continue;
        }
        match &r.action {
            RecAction::Changes { changes: c } => changes.extend(c.iter().cloned()),
            _ => result.items.push(ResultItem { kind: "info".into(), label: r.title.clone(), ok: false, detail: Some("This recommendation has no automatic change; follow the guidance on its card.".into()) }),
        }
    }
    if changes.is_empty() {
        return result;
    }
    match journal.apply("manual", &changes) {
        Ok(outcomes) => result.add_outcomes("setting", &outcomes),
        Err((e, outcomes)) => {
            result.add_outcomes("setting", &outcomes);
            result.items.push(ResultItem { kind: "info".into(), label: "All changes in this run were rolled back".into(), ok: false, detail: Some(e.to_string()) });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::fake::FakeOps;
    use crate::sys::{GUID_BALANCED, GUID_HIGH};
    use std::sync::Arc;

    fn setup() -> (tempfile::TempDir, Arc<FakeOps>, Journal) {
        let d = tempfile::tempdir().unwrap();
        let ops = Arc::new(FakeOps::with_default_plans());
        let j = Journal::new(DataDir::new(d.path()).unwrap(), ops.clone());
        (d, ops, j)
    }

    #[test]
    fn profile_plan_skips_noops() {
        let (_d, ops, _j) = setup();
        let comp = crate::profiles::default_profiles().remove(1);
        // Nothing set yet: only the power plan differs.
        let p = profile_plan(ops.as_ref(), &comp).unwrap();
        assert_eq!(p.len(), 1);
        ops.reg_set(Hive::CurrentUser, GAMEDVR, "AppCaptureEnabled", &RegData::Dword(1)).unwrap();
        assert_eq!(profile_plan(ops.as_ref(), &comp).unwrap().len(), 2);
    }

    #[test]
    fn maximum_falls_back_to_high_when_ultimate_unavailable() {
        struct NoUltimate(FakeOps);
        impl SystemOps for NoUltimate {
            fn reg_get(&self, h: Hive, p: &str, n: &str) -> Result<Option<RegData>> { self.0.reg_get(h, p, n) }
            fn reg_set(&self, h: Hive, p: &str, n: &str, d: &RegData) -> Result<()> { self.0.reg_set(h, p, n, d) }
            fn reg_delete_value(&self, h: Hive, p: &str, n: &str) -> Result<()> { self.0.reg_delete_value(h, p, n) }
            fn reg_values(&self, h: Hive, p: &str) -> Result<Vec<(String, RegData)>> { self.0.reg_values(h, p) }
            fn reg_subkeys(&self, h: Hive, p: &str) -> Result<Vec<String>> { self.0.reg_subkeys(h, p) }
            fn power_plans(&self) -> Result<Vec<crate::sys::PowerPlan>> { self.0.power_plans() }
            fn power_set_active(&self, g: &str) -> Result<()> { self.0.power_set_active(g) }
            fn power_ensure_ultimate(&self) -> Result<crate::sys::PowerPlan> { Err(Error::Unsupported("no".into())) }
        }
        let ops = NoUltimate(FakeOps::with_default_plans());
        let max = crate::profiles::default_profiles().remove(2);
        match &profile_plan(&ops, &max).unwrap()[0] {
            PlannedChange::PowerPlan { target, .. } => assert_eq!(*target, PlanKind::HighPerformance),
            _ => panic!(),
        }
    }

    #[test]
    fn optimize_now_applies_only_safe_auto_and_counts_real_work() {
        let (_d, ops, j) = setup();
        ops.reg_set(Hive::CurrentUser, analysis::GAMEBAR, "AutoGameModeEnabled", &RegData::Dword(0)).unwrap();
        let snap = Snapshot { game_mode: Some(0), temp_bytes: 5 << 30, ..Default::default() };
        let recs = analysis::analyze(&snap);
        let res = apply_recommendations(&j, &recs, None, "Optimize now");
        assert_eq!(res.applied, 1);
        assert_eq!(res.failed, 0);
        assert_eq!(ops.reg_get(Hive::CurrentUser, analysis::GAMEBAR, "AutoGameModeEnabled").unwrap(), Some(RegData::Dword(1)));
        // Running it again changes nothing and reports nothing applied.
        let again = apply_recommendations(&j, &recs, None, "Optimize now");
        assert_eq!((again.applied, again.already_set), (0, 1));
        // The temp-files recommendation was never "applied".
        assert!(res.items.iter().all(|i| i.kind == "setting"));
    }

    #[test]
    fn failed_optimization_is_rolled_back_and_reported() {
        let (_d, ops, j) = setup();
        let snap = Snapshot {
            game_mode: Some(0),
            app_capture: Some(1),
            power_plan: Some(crate::sys::PowerPlan { guid: crate::sys::GUID_SAVER.into(), name: "Power saver".into(), active: true, kind: PlanKind::PowerSaver }),
            has_high_perf_plan: true,
            ..Default::default()
        };
        ops.fail_writes_to("AppCaptureEnabled");
        let res = apply_recommendations(&j, &analysis::analyze(&snap), None, "Optimize now");
        assert_eq!(res.applied, 0);
        assert!(res.failed >= 1);
        assert_eq!(ops.active_guid().unwrap(), GUID_BALANCED);
        assert_eq!(j.active_count(), 0);
        let _ = GUID_HIGH;
    }

    #[test]
    fn gpu_preference_validates_paths() {
        assert!(gpu_pref_change("relative.exe", true).is_err());
        assert!(gpu_pref_change("C:\\Games\\a.txt", true).is_err());
        #[cfg(windows)]
        assert!(gpu_pref_change("C:\\Games\\a.exe", true).is_ok());
    }
}
