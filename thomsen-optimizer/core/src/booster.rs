//! Game Booster: activates a gaming profile when a configured game starts and
//! restores everything when the last game closes.
//!
//! The state machine is deliberately small and pure: [`Booster::tick`] is fed
//! the games currently running and returns events. All Windows changes go
//! through the journal (scope `session:<id>`), so an interrupted session is
//! undone on the next app start (`Journal::recover_after_crash`).
//!
//! Multiple games at once: the first game to start decides the profile; games
//! that start later join the same session. The session ends (and settings are
//! restored) only when **all** of its games have closed.

use crate::journal::{ApplyOutcome, Journal};
use crate::optimizer::profile_plan;
use crate::profiles::{find, ProfileConfig};
use crate::sys::SystemOps;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Ticks a game must be absent before it counts as closed (launcher
/// relaunches and loading screens briefly drop the process).
const MISSING_TICKS_TO_END: u8 = 2;

#[derive(Debug, Clone)]
pub struct RunningGame {
    pub id: String,
    pub name: String,
    pub profile_id: Option<String>,
    pub auto_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoosterStatus {
    pub active: bool,
    pub profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub games: Vec<String>,
    pub change_count: usize,
    pub manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BoosterEvent {
    #[serde(rename_all = "camelCase")]
    Activated { game: String, profile_name: String, outcomes: Vec<ApplyOutcome> },
    #[serde(rename_all = "camelCase")]
    Failed { game: String, message: String },
    #[serde(rename_all = "camelCase")]
    Restored { restored: usize, failed: Vec<String> },
}

struct Session {
    scope: String,
    profile: ProfileConfig,
    started_at: DateTime<Utc>,
    games: Vec<(String, String)>,
    manual: bool,
    missing_ticks: u8,
}

#[derive(Default)]
pub struct Booster {
    session: Option<Session>,
    /// Games whose activation failed; not retried until they have closed.
    failed: HashSet<String>,
}

impl Booster {
    pub fn scope(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.scope.clone())
    }

    pub fn live_scopes(&self) -> HashSet<String> {
        self.scope().into_iter().collect()
    }

    pub fn game_running(&self) -> bool {
        self.session.as_ref().is_some_and(|s| !s.games.is_empty())
    }

    pub fn status(&self, journal: &Journal) -> BoosterStatus {
        match &self.session {
            Some(s) => BoosterStatus {
                active: true,
                profile_id: Some(s.profile.id.clone()),
                profile_name: Some(s.profile.name.clone()),
                started_at: Some(s.started_at),
                games: s.games.iter().map(|g| g.1.clone()).collect(),
                change_count: journal.list().iter().filter(|r| r.scope == s.scope && r.reversible() && r.status != crate::journal::ChangeStatus::Restored).count(),
                manual: s.manual,
            },
            None => BoosterStatus { active: false, profile_id: None, profile_name: None, started_at: None, games: vec![], change_count: 0, manual: false },
        }
    }

    fn start(&mut self, ops: &dyn SystemOps, journal: &Journal, profile: &ProfileConfig, games: Vec<(String, String)>, manual: bool) -> Result<Vec<ApplyOutcome>, String> {
        let scope = format!("session:{}", uuid::Uuid::new_v4());
        let plan = profile_plan(ops, profile).map_err(|e| e.to_string())?;
        let outcomes = if plan.is_empty() { vec![] } else { journal.apply(&scope, &plan).map_err(|(e, _)| e.to_string())? };
        self.session = Some(Session { scope, profile: profile.clone(), started_at: Utc::now(), games, manual, missing_ticks: 0 });
        Ok(outcomes)
    }

    /// Called every detection cycle with the currently running games.
    pub fn tick(&mut self, ops: &dyn SystemOps, journal: &Journal, profiles: &[ProfileConfig], default_profile: &str, auto_enabled: bool, restore_on_exit: bool, running: &[RunningGame]) -> Vec<BoosterEvent> {
        let mut events = Vec::new();
        let running_ids: HashSet<&str> = running.iter().map(|g| g.id.as_str()).collect();
        self.failed.retain(|id| running_ids.contains(id.as_str()));
        let eligible: Vec<&RunningGame> = running.iter().filter(|g| g.auto_boost && auto_enabled && !self.failed.contains(&g.id)).collect();

        // Start or extend.
        if let Some(first) = eligible.first() {
            match &mut self.session {
                None => {
                    let pid = first.profile_id.as_deref().unwrap_or(default_profile);
                    let profile = find(profiles, pid).or_else(|| find(profiles, "balanced")).cloned();
                    if let Some(profile) = profile {
                        let games: Vec<(String, String)> = eligible.iter().map(|g| (g.id.clone(), g.name.clone())).collect();
                        match self.start(ops, journal, &profile, games, false) {
                            Ok(outcomes) => events.push(BoosterEvent::Activated { game: first.name.clone(), profile_name: profile.name.clone(), outcomes }),
                            Err(m) => {
                                for g in &eligible {
                                    self.failed.insert(g.id.clone());
                                }
                                events.push(BoosterEvent::Failed { game: first.name.clone(), message: m });
                            }
                        }
                    }
                }
                Some(s) => {
                    s.missing_ticks = 0;
                    for g in &eligible {
                        if !s.games.iter().any(|x| x.0 == g.id) {
                            s.games.push((g.id.clone(), g.name.clone()));
                        }
                    }
                }
            }
        }

        // Track games leaving, and end the session when none remain.
        if let Some(s) = &mut self.session {
            if !s.games.is_empty() {
                let any_running = s.games.iter().any(|g| running_ids.contains(g.0.as_str()));
                if any_running {
                    s.games.retain(|g| running_ids.contains(g.0.as_str()));
                    s.missing_ticks = 0;
                } else {
                    s.missing_ticks += 1;
                    if s.missing_ticks >= MISSING_TICKS_TO_END {
                        let scope = s.scope.clone();
                        self.session = None;
                        if restore_on_exit {
                            let r = journal.revert_scope(&scope);
                            events.push(BoosterEvent::Restored { restored: r.restored, failed: r.failed });
                        }
                    }
                }
            }
        }
        events
    }

    /// User-initiated activation (Game Booster page / dashboard button).
    pub fn activate_manual(&mut self, ops: &dyn SystemOps, journal: &Journal, profile: &ProfileConfig) -> Result<Vec<ApplyOutcome>, String> {
        if self.session.is_some() {
            return Err("Gaming mode is already active. Restore it first to switch profiles.".into());
        }
        self.start(ops, journal, profile, vec![], true)
    }

    /// User-initiated restore of the active session.
    pub fn deactivate(&mut self, journal: &Journal) -> BoosterEvent {
        let scope = self.session.take().map(|s| s.scope);
        match scope {
            Some(sc) => {
                let r = journal.revert_scope(&sc);
                BoosterEvent::Restored { restored: r.restored, failed: r.failed }
            }
            None => BoosterEvent::Restored { restored: 0, failed: vec![] },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::GAMEDVR;
    use crate::profiles::default_profiles;
    use crate::store::DataDir;
    use crate::sys::fake::FakeOps;
    use crate::sys::{Hive, RegData, GUID_BALANCED, GUID_HIGH};
    use std::sync::Arc;

    struct Fx {
        _d: tempfile::TempDir,
        ops: Arc<FakeOps>,
        j: Journal,
        profiles: Vec<ProfileConfig>,
    }

    fn fx() -> Fx {
        let d = tempfile::tempdir().unwrap();
        let ops = Arc::new(FakeOps::with_default_plans());
        ops.reg_set(Hive::CurrentUser, GAMEDVR, "AppCaptureEnabled", &RegData::Dword(1)).unwrap();
        let j = Journal::new(DataDir::new(d.path()).unwrap(), ops.clone());
        Fx { _d: d, ops, j, profiles: default_profiles() }
    }

    fn game(id: &str, profile: &str) -> RunningGame {
        RunningGame { id: id.into(), name: id.to_uppercase(), profile_id: Some(profile.into()), auto_boost: true }
    }

    fn tick(f: &Fx, b: &mut Booster, running: &[RunningGame]) -> Vec<BoosterEvent> {
        b.tick(f.ops.as_ref(), &f.j, &f.profiles, "balanced", true, true, running)
    }

    #[test]
    fn activates_on_launch_and_restores_after_exit() {
        let f = fx();
        let mut b = Booster::default();
        assert!(tick(&f, &mut b, &[]).is_empty());
        let ev = tick(&f, &mut b, &[game("cs2", "competitive")]);
        assert!(matches!(ev[0], BoosterEvent::Activated { .. }));
        assert_eq!(f.ops.active_guid().unwrap(), GUID_HIGH);
        assert_eq!(f.ops.reg_get(Hive::CurrentUser, GAMEDVR, "AppCaptureEnabled").unwrap(), Some(RegData::Dword(0)));
        assert!(b.status(&f.j).active && b.game_running());
        // Still running: nothing new happens.
        assert!(tick(&f, &mut b, &[game("cs2", "competitive")]).is_empty());
        // One missing tick is tolerated (loading screen / launcher relaunch)...
        assert!(tick(&f, &mut b, &[]).is_empty());
        assert_eq!(f.ops.active_guid().unwrap(), GUID_HIGH);
        // ...the second ends the session and restores everything.
        let ev = tick(&f, &mut b, &[]);
        assert!(matches!(&ev[0], BoosterEvent::Restored { restored, .. } if *restored == 2));
        assert_eq!(f.ops.active_guid().unwrap(), GUID_BALANCED);
        assert_eq!(f.ops.reg_get(Hive::CurrentUser, GAMEDVR, "AppCaptureEnabled").unwrap(), Some(RegData::Dword(1)));
        assert!(!b.status(&f.j).active);
    }

    #[test]
    fn multiple_games_share_one_session_until_all_close() {
        let f = fx();
        let mut b = Booster::default();
        tick(&f, &mut b, &[game("a", "competitive")]);
        tick(&f, &mut b, &[game("a", "competitive"), game("b", "maximum")]);
        assert_eq!(b.status(&f.j).games.len(), 2);
        // The profile stays the first game's; the second does not re-apply.
        assert_eq!(b.status(&f.j).profile_id.as_deref(), Some("competitive"));
        // A closes, B keeps the session alive.
        for _ in 0..3 {
            assert!(tick(&f, &mut b, &[game("b", "maximum")]).is_empty());
        }
        assert_eq!(f.ops.active_guid().unwrap(), GUID_HIGH);
        tick(&f, &mut b, &[]);
        let ev = tick(&f, &mut b, &[]);
        assert!(matches!(ev[0], BoosterEvent::Restored { .. }));
    }

    #[test]
    fn games_without_auto_boost_or_with_it_disabled_globally_do_nothing() {
        let f = fx();
        let mut b = Booster::default();
        let mut g = game("a", "competitive");
        g.auto_boost = false;
        assert!(tick(&f, &mut b, &[g]).is_empty());
        let ev = b.tick(f.ops.as_ref(), &f.j, &f.profiles, "balanced", false, true, &[game("a", "competitive")]);
        assert!(ev.is_empty() && !b.status(&f.j).active);
    }

    #[test]
    fn failed_activation_leaves_nothing_behind_and_is_not_retried() {
        let f = fx();
        f.ops.fail_writes_to("AppCaptureEnabled");
        let mut b = Booster::default();
        let ev = tick(&f, &mut b, &[game("a", "competitive")]);
        assert!(matches!(ev[0], BoosterEvent::Failed { .. }));
        assert_eq!(f.ops.active_guid().unwrap(), GUID_BALANCED);
        assert_eq!(f.j.active_count(), 0);
        assert!(tick(&f, &mut b, &[game("a", "competitive")]).is_empty());
    }

    #[test]
    fn manual_activation_and_restore() {
        let f = fx();
        let mut b = Booster::default();
        let comp = f.profiles[1].clone();
        b.activate_manual(f.ops.as_ref(), &f.j, &comp).unwrap();
        assert!(b.status(&f.j).manual && b.status(&f.j).change_count == 2);
        assert!(b.activate_manual(f.ops.as_ref(), &f.j, &comp).is_err());
        // A manual session with no game does not end by itself.
        for _ in 0..5 {
            assert!(tick(&f, &mut b, &[]).is_empty());
        }
        assert!(matches!(b.deactivate(&f.j), BoosterEvent::Restored { restored: 2, .. }));
        assert_eq!(f.ops.active_guid().unwrap(), GUID_BALANCED);
    }

    #[test]
    fn restore_on_exit_can_be_turned_off() {
        let f = fx();
        let mut b = Booster::default();
        tick(&f, &mut b, &[game("a", "competitive")]);
        for _ in 0..2 {
            b.tick(f.ops.as_ref(), &f.j, &f.profiles, "balanced", true, false, &[]);
        }
        assert!(!b.status(&f.j).active);
        // Changes stay in the journal so the user can still restore them.
        assert_eq!(f.j.active_count(), 2);
    }
}
