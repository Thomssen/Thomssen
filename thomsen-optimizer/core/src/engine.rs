//! The application engine: owns all state and exposes one method per user
//! action. The Tauri shell is a thin layer of `#[tauri::command]`s over this.

use crate::analysis::{self, Recommendation};
use crate::booster::{Booster, BoosterEvent, BoosterStatus, RunningGame};
use crate::cleaner::{self, CleanReport, CleanerEnv, Scan, ScanSummary};
use crate::error::{Error, Result};
use crate::fps::{self, Capture, LiveFps};
use crate::games::{launchers, process_matches, Detected, Game, GameSettings, Library};
use crate::history::{compare, Comparison, FpsSession, History};
use crate::hwinfo::{self, SystemInfo};
use crate::journal::{ChangeRecord, Journal, PlannedChange, RestoreReport};
use crate::monitor::{GpuProbe, Monitor, Sample};
use crate::optimizer::{self, OptimizationResult};
use crate::procs::{self, ClosePlan, CloseResult, ProcGroup};
use crate::profiles::{valid_exe_name, ProfileConfig};
use crate::settings::Settings;
use crate::startup::{self, StartupEntry};
use crate::store::DataDir;
use crate::sys::{self, PowerPlan, SystemOps};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

const SETTINGS_FILE: &str = "settings.json";
const GAME_SCAN_TTL: Duration = Duration::from_secs(300);

/// Events pushed to the UI by the background loop.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum EngineEvent {
    Sample(Sample),
    Booster(BoosterEvent),
    BoosterStatus(BoosterStatus),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub power_plan: Option<PowerPlan>,
    pub booster: BoosterStatus,
    pub running_games: Vec<String>,
    pub restorable_changes: usize,
    pub fps: Option<f64>,
    pub elevated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReport {
    pub recommendations: Vec<Recommendation>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub cpu_avg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeRequest {
    pub recommendation_ids: Vec<String>,
    pub close: Vec<ClosePlan>,
    pub clean_scan_id: Option<String>,
    pub clean_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FpsStatus {
    pub presentmon_path: Option<String>,
    pub presentmon_ok: bool,
    pub capturing: bool,
    pub elevated: bool,
}

pub struct Engine {
    pub data: DataDir,
    pub ops: Arc<dyn SystemOps>,
    pub journal: Journal,
    pub library: Library,
    pub history: History,
    settings: RwLock<Settings>,
    monitor: Mutex<Monitor>,
    gpu: Mutex<GpuProbe>,
    booster: Mutex<Booster>,
    capture: Mutex<Option<Capture>>,
    capture_meta: Mutex<Option<(String, Option<String>)>>, // (game name, profile id when started)
    scan: Mutex<Option<Scan>>,
    sysinfo: Mutex<Option<SystemInfo>>,
    detected: Mutex<(Option<Instant>, Vec<Detected>)>,
    last_sample: Mutex<Option<Sample>>,
    cpu_avg: Mutex<f32>,
    temp_cache: Mutex<Option<(Instant, u64)>>,
    ui_visible: AtomicBool,
    window_hidden: AtomicBool,
    ui_monitoring: AtomicBool,
    pub stop: AtomicBool,
    wake: (Mutex<()>, Condvar),
}

impl Engine {
    pub fn new(data: DataDir, ops: Arc<dyn SystemOps>) -> Arc<Engine> {
        let mut settings: Settings = data.read_json(SETTINGS_FILE);
        settings.sanitize();
        let journal = Journal::new(data.clone(), ops.clone());
        // Undo anything an interrupted game session left behind.
        let rep = journal.recover_after_crash(&Default::default());
        if rep.restored > 0 || !rep.failed.is_empty() {
            log::warn!("startup recovery: restored {} change(s), {} failed", rep.restored, rep.failed.len());
        }
        let mut monitor = Monitor::new();
        monitor.temps_enabled = settings.temperature_monitoring;
        let history = History::new(data.clone());
        history.prune(settings.history_retention_days);
        Arc::new(Engine {
            library: Library::new(data.clone()),
            history,
            journal,
            ops,
            settings: RwLock::new(settings),
            monitor: Mutex::new(monitor),
            gpu: Mutex::new(GpuProbe::default()),
            booster: Mutex::new(Booster::default()),
            capture: Mutex::new(None),
            capture_meta: Mutex::new(None),
            scan: Mutex::new(None),
            sysinfo: Mutex::new(None),
            detected: Mutex::new((None, vec![])),
            last_sample: Mutex::new(None),
            cpu_avg: Mutex::new(0.0),
            temp_cache: Mutex::new(None),
            ui_visible: AtomicBool::new(true),
            window_hidden: AtomicBool::new(false),
            ui_monitoring: AtomicBool::new(true),
            stop: AtomicBool::new(false),
            wake: (Mutex::new(()), Condvar::new()),
            data,
        })
    }

    // -- settings ----------------------------------------------------------------

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn update_settings(&self, mut s: Settings) -> Result<Settings> {
        s.sanitize();
        // Profiles: unique ids, plain exe names, built-ins keep their identity.
        let mut seen = std::collections::HashSet::new();
        for p in &mut s.profiles {
            if p.id.is_empty() || !seen.insert(p.id.clone()) {
                return Err(Error::Invalid("Profile ids must be unique.".into()));
            }
            p.name = p.name.trim().chars().take(40).collect();
            if p.name.is_empty() {
                return Err(Error::Invalid("A profile needs a name.".into()));
            }
            p.description = p.description.chars().take(400).collect();
            p.tradeoffs = p.tradeoffs.chars().take(400).collect();
            p.close_apps.retain(|a| valid_exe_name(a));
            p.close_apps.dedup();
            p.close_apps.truncate(40);
            if crate::profiles::default_profiles().iter().any(|d| d.id == p.id) {
                p.builtin = true;
            }
        }
        if let Some(p) = &s.presentmon_path {
            fps::validate_presentmon(std::path::Path::new(p))?;
        }
        self.data.write_json(SETTINGS_FILE, &s)?;
        self.monitor.lock().unwrap().temps_enabled = s.temperature_monitoring;
        *self.settings.write().unwrap() = s.clone();
        self.wake();
        Ok(s)
    }

    pub fn set_ui_state(&self, visible: bool, monitoring: bool) {
        self.ui_visible.store(visible, Ordering::Relaxed);
        self.ui_monitoring.store(monitoring, Ordering::Relaxed);
        self.wake();
    }

    /// Set by the app shell when the window is hidden to the tray or minimised.
    pub fn set_window_hidden(&self, hidden: bool) {
        self.window_hidden.store(hidden, Ordering::Relaxed);
        self.wake();
    }

    fn visible(&self) -> bool {
        self.ui_visible.load(Ordering::Relaxed) && !self.window_hidden.load(Ordering::Relaxed)
    }

    fn wake(&self) {
        self.wake.1.notify_all();
    }

    // -- background loop -----------------------------------------------------------

    fn gaming(&self) -> bool {
        self.booster.lock().unwrap().game_running()
    }

    /// Adaptive cadence: fast only while a monitoring view is on screen; slow
    /// when other pages are shown, much slower when the window is hidden, and
    /// never faster than 2 s while a game is running.
    pub fn sample_interval(&self) -> Duration {
        let s = self.settings.read().unwrap();
        let gaming = self.gaming();
        let ms = if !self.visible() {
            15_000
        } else if self.ui_monitoring.load(Ordering::Relaxed) {
            if gaming { (s.refresh_ms as u64).max(2000) } else { s.refresh_ms as u64 }
        } else if gaming {
            10_000
        } else {
            5_000
        };
        Duration::from_millis(ms)
    }

    fn detect_interval(&self) -> Option<Duration> {
        if !self.settings.read().unwrap().auto_detect_games {
            return None;
        }
        Some(if self.gaming() { Duration::from_secs(5) } else { Duration::from_secs(3) })
    }

    /// Runs until `stop` is set. `emit` receives events; `Sample` events are
    /// only produced while the window is visible.
    pub fn run(&self, emit: impl Fn(EngineEvent)) {
        let mut next_sample = Instant::now();
        let mut next_detect = Instant::now() + Duration::from_secs(2);
        while !self.stop.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= next_sample {
                let visible = self.visible();
                let sample = self.take_sample();
                if visible {
                    emit(EngineEvent::Sample(sample));
                }
                next_sample = Instant::now() + self.sample_interval();
            }
            if now >= next_detect {
                if self.detect_interval().is_some() {
                    for ev in self.detection_tick() {
                        emit(EngineEvent::Booster(ev));
                    }
                    emit(EngineEvent::BoosterStatus(self.booster_status()));
                }
                next_detect = Instant::now() + self.detect_interval().unwrap_or(Duration::from_secs(10));
            }
            let due = next_sample.min(next_detect).saturating_duration_since(Instant::now()).max(Duration::from_millis(50));
            let guard = self.wake.0.lock().unwrap();
            let _ = self.wake.1.wait_timeout(guard, due.min(Duration::from_secs(2))).unwrap();
        }
    }

    pub fn take_sample(&self) -> Sample {
        let visible = self.visible();
        let monitoring = self.ui_monitoring.load(Ordering::Relaxed);
        let mut sample = self.monitor.lock().unwrap().sample();
        {
            let mut a = self.cpu_avg.lock().unwrap();
            *a = if *a == 0.0 { sample.cpu_usage } else { *a * 0.9 + sample.cpu_usage * 0.1 };
        }
        // GPU: probed outside the monitor lock, never while the window is
        // hidden, and less often when no monitoring page is on screen.
        let gpu_every = if monitoring { Duration::from_millis(self.settings.read().unwrap().refresh_ms as u64).max(Duration::from_secs(2)) } else { Duration::from_secs(15) };
        {
            let mut g = self.gpu.lock().unwrap();
            sample.gpu = if visible { g.get(gpu_every) } else { g.cached() };
        }
        *self.last_sample.lock().unwrap() = Some(sample.clone());
        sample
    }

    pub fn latest_sample(&self) -> Option<Sample> {
        self.last_sample.lock().unwrap().clone()
    }

    // -- games -----------------------------------------------------------------------

    fn detected(&self, force: bool) -> Vec<Detected> {
        let mut g = self.detected.lock().unwrap();
        if force || g.0.map_or(true, |t| t.elapsed() > GAME_SCAN_TTL) {
            g.1 = launchers::detect_all(self.ops.as_ref());
            g.0 = Some(Instant::now());
        }
        g.1.clone()
    }

    pub fn games(&self, rescan: bool) -> Vec<Game> {
        let mut games = self.library.build(self.detected(rescan));
        let procs = {
            let mut m = self.monitor.lock().unwrap();
            procs::refresh(&mut m.sys);
            procs::list(&m.sys, false, false)
        };
        for g in &mut games {
            let matching: Vec<&procs::ProcInfo> = procs.iter().filter(|p| process_matches(g, p.exe.as_deref(), &p.name)).collect();
            g.running_pids = matching.iter().map(|p| p.pid).collect();
            g.memory_bytes = matching.iter().map(|p| p.memory_bytes).sum();
            g.running = !g.running_pids.is_empty();
        }
        games
    }

    fn detection_tick(&self) -> Vec<BoosterEvent> {
        let settings = self.settings();
        let running: Vec<RunningGame> = self
            .games(false)
            .into_iter()
            .filter(|g| g.running)
            .map(|g| RunningGame { id: g.id, name: g.name, profile_id: g.settings.profile_id, auto_boost: g.settings.auto_boost })
            .collect();
        let events = self.booster.lock().unwrap().tick(self.ops.as_ref(), &self.journal, &settings.profiles, &settings.default_profile_id, settings.auto_boost, settings.restore_when_game_closes, &running);
        for e in &events {
            match e {
                BoosterEvent::Failed { game, message } => log::warn!("game booster could not activate for {game}: {message}"),
                BoosterEvent::Restored { restored, failed } if !failed.is_empty() => log::warn!("game booster restored {restored}, {} failed", failed.len()),
                _ => {}
            }
        }
        events
    }

    pub fn add_game(&self, name: &str, exe: &str) -> Result<()> {
        self.library.add_custom(name, exe).map(|_| ())
    }

    pub fn remove_game(&self, id: &str) -> Result<()> {
        self.library.remove_custom(id)
    }

    pub fn update_game(&self, id: &str, new: GameSettings) -> Result<GameSettings> {
        if let Some(p) = &new.profile_id {
            if !self.settings().profiles.iter().any(|x| &x.id == p) {
                return Err(Error::Invalid("That profile does not exist.".into()));
            }
        }
        self.library.update_settings(id, |s| *s = new)
    }

    pub fn set_gpu_preference(&self, game_id: &str, high: bool) -> Result<Vec<crate::journal::ApplyOutcome>> {
        let game = self.games(false).into_iter().find(|g| g.id == game_id).ok_or_else(|| Error::NotFound("Game not found.".into()))?;
        let exe = game.exe_path.ok_or_else(|| Error::Invalid("This game's executable is unknown; add it manually first.".into()))?;
        let change = optimizer::gpu_pref_change(&exe, high)?;
        self.journal.apply("manual", &[change]).map_err(|(e, _)| e)
    }

    // -- game booster ------------------------------------------------------------------

    pub fn booster_status(&self) -> BoosterStatus {
        self.booster.lock().unwrap().status(&self.journal)
    }

    /// Manually activates a profile. Closing apps listed in the profile only
    /// happens when `close_apps` is true, and never for protected processes.
    pub fn activate_profile(&self, profile_id: &str, close_apps: bool) -> Result<OptimizationResult> {
        let settings = self.settings();
        let profile: ProfileConfig = crate::profiles::find(&settings.profiles, profile_id).cloned().ok_or_else(|| Error::NotFound("Profile not found.".into()))?;
        let mut result = OptimizationResult::new(&format!("{} profile activated", profile.name));
        let outcomes = self.booster.lock().unwrap().activate_manual(self.ops.as_ref(), &self.journal, &profile).map_err(Error::Busy)?;
        result.add_outcomes("setting", &outcomes);
        result.add("profile", format!("Profile \"{}\" is active", profile.name), true, Some("Changes are temporary; restore them from the Game Booster page.".into()));
        if close_apps && !profile.close_apps.is_empty() {
            let plans = self.processes().into_iter().filter(|g| !g.protected && profile.close_apps.iter().any(|a| a.eq_ignore_ascii_case(&g.name))).flat_map(|g| g.members).collect::<Vec<_>>();
            self.close_into(&plans, &mut result);
        }
        optimizer::save_result(&self.data, &result);
        Ok(result)
    }

    pub fn deactivate_profile(&self) -> BoosterEvent {
        self.booster.lock().unwrap().deactivate(&self.journal)
    }

    // -- processes ---------------------------------------------------------------------

    pub fn processes(&self) -> Vec<ProcGroup> {
        let gaming = self.gaming();
        let list = {
            let mut m = self.monitor.lock().unwrap();
            procs::refresh(&mut m.sys);
            procs::list(&m.sys, gaming, true)
        };
        procs::group(&list)
    }

    fn close_into(&self, plans: &[ClosePlan], result: &mut OptimizationResult) -> Vec<CloseResult> {
        let gaming = self.gaming();
        let res = {
            let mut m = self.monitor.lock().unwrap();
            procs::close(&mut m.sys, plans, gaming)
        };
        // Summarise per program name.
        let mut by_name: std::collections::BTreeMap<String, (usize, usize, Option<String>)> = Default::default();
        for r in &res {
            let e = by_name.entry(r.name.clone()).or_default();
            if r.ok {
                e.0 += 1;
            } else {
                e.1 += 1;
                e.2 = r.message.clone();
            }
        }
        for (name, (ok, bad, msg)) in by_name {
            if ok > 0 {
                result.add("process", format!("Closed {name}{}", if ok > 1 { format!(" ({ok} processes)") } else { String::new() }), true, Some("Apps with open windows were asked to close, so they could offer to save.".into()));
                self.journal.log_irreversible("process", "process", &format!("Closed {name}"), "Not restorable - start the program again if needed.");
            }
            if bad > 0 {
                result.add("process", format!("Could not close {name}"), false, msg);
            }
        }
        res
    }

    pub fn close_processes(&self, plans: Vec<ClosePlan>) -> OptimizationResult {
        let mut result = OptimizationResult::new("Background apps closed");
        self.close_into(&plans, &mut result);
        optimizer::save_result(&self.data, &result);
        result
    }

    // -- analysis & optimization ------------------------------------------------------------

    fn temp_bytes(&self) -> u64 {
        let mut c = self.temp_cache.lock().unwrap();
        if let Some((t, v)) = *c {
            if t.elapsed() < Duration::from_secs(300) {
                return v;
            }
        }
        let v = cleaner::quick_temp_bytes(&CleanerEnv::from_system(), self.settings().cleaner_min_age_hours as u64);
        *c = Some((Instant::now(), v));
        v
    }

    pub fn analyze(&self) -> AnalysisReport {
        let gaming = self.gaming();
        let cpu_avg = *self.cpu_avg.lock().unwrap();
        let info = self.system_info();
        let gpu_names: Vec<String> = info.as_ref().map(|i| i.gpus.iter().map(|g| g.name.clone()).collect()).unwrap_or_default();
        let sysdrive = info.as_ref().and_then(|i| {
            let wd = procs::windows_dir()?;
            let prefix: String = wd.chars().take(3).collect::<String>().to_ascii_lowercase();
            i.drives.iter().find(|d| d.mount.to_ascii_lowercase().starts_with(&prefix)).map(|d| (d.free_bytes, d.total_bytes, d.kind.clone()))
        });
        let startup_enabled = startup::enabled_count(self.ops.as_ref());
        let temp = self.temp_bytes();
        let snapshot = {
            let mut m = self.monitor.lock().unwrap();
            procs::refresh(&mut m.sys);
            let list = procs::list(&m.sys, gaming, false);
            optimizer::gather_snapshot(self.ops.as_ref(), &m.sys, &list, cpu_avg, startup_enabled, temp, gpu_names, sysdrive)
        };
        AnalysisReport { recommendations: analysis::analyze(&snapshot), generated_at: chrono::Utc::now(), cpu_avg }
    }

    /// "Optimize Now": the recommended safe *settings* only - never closes
    /// apps or deletes files.
    pub fn optimize_now(&self) -> Result<OptimizationResult> {
        self.maybe_restore_point();
        let report = self.analyze();
        let result = optimizer::apply_recommendations(&self.journal, &report.recommendations, None, "Optimization completed");
        optimizer::save_result(&self.data, &result);
        Ok(result)
    }

    /// The full Optimize page flow: chosen recommendations, chosen apps, chosen cleanup.
    pub fn optimize_selected(&self, req: OptimizeRequest) -> Result<OptimizationResult> {
        if self.settings().auto_restore_point && !req.recommendation_ids.is_empty() {
            self.maybe_restore_point();
        }
        let report = self.analyze();
        let mut result = optimizer::apply_recommendations(&self.journal, &report.recommendations, Some(&req.recommendation_ids), "Optimization completed");
        if !req.close.is_empty() {
            self.close_into(&req.close, &mut result);
        }
        if let (Some(id), false) = (&req.clean_scan_id, req.clean_categories.is_empty()) {
            let rep = self.clean(id, &req.clean_categories)?;
            result.bytes_freed += rep.total_freed;
            result.add("cleanup", format!("{} of temporary files removed", crate::analysis::gib(rep.total_freed)), rep.total_deleted > 0 || rep.total_freed > 0, Some(format!("{} files deleted, {} skipped (in use or changed).", rep.total_deleted, rep.total_skipped)));
        }
        optimizer::save_result(&self.data, &result);
        Ok(result)
    }

    pub fn apply_recommendation(&self, id: &str) -> Result<OptimizationResult> {
        let report = self.analyze();
        if !report.recommendations.iter().any(|r| r.id == id) {
            return Err(Error::NotFound("That recommendation no longer applies.".into()));
        }
        let result = optimizer::apply_recommendations(&self.journal, &report.recommendations, Some(&[id.to_string()]), "Recommendation applied");
        optimizer::save_result(&self.data, &result);
        Ok(result)
    }

    pub fn results(&self) -> Vec<OptimizationResult> {
        optimizer::load_results(&self.data)
    }

    fn maybe_restore_point(&self) {
        if self.settings().auto_restore_point {
            if let Err(e) = self.create_restore_point() {
                log::warn!("restore point not created: {e}");
            }
        }
    }

    /// Creates a Windows System Restore point (needs administrator approval
    /// via a UAC prompt). Windows allows one per 24 h by default.
    pub fn create_restore_point(&self) -> Result<()> {
        #[cfg(not(windows))]
        {
            Err(Error::Unsupported("System Restore is only available on Windows.".into()))
        }
        #[cfg(windows)]
        {
            let script = "$p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-Command','try { Checkpoint-Computer -Description Thomsen-Optimizer -RestorePointType MODIFY_SETTINGS -ErrorAction Stop; exit 0 } catch { exit 1 }'; exit $p.ExitCode";
            sys::proc_util::powershell(script, Duration::from_secs(180)).map(|_| ()).map_err(|e| match e {
                Error::PermissionDenied(m) => Error::PermissionDenied(m),
                _ => Error::Command("Windows did not create a restore point. System Protection may be off, the UAC prompt was declined, or a restore point was already created in the last 24 hours.".into()),
            })
        }
    }

    // -- restore -------------------------------------------------------------------------

    pub fn history_list(&self) -> Vec<ChangeRecord> {
        self.journal.list()
    }

    pub fn restore_one(&self, id: &str) -> Result<()> {
        self.journal.revert(id)
    }

    pub fn restore_all(&self) -> RestoreReport {
        // End the active gaming session first (restores its changes), then
        // everything else in the journal; report both together.
        let ended = self.booster.lock().unwrap().deactivate(&self.journal);
        let mut rep = self.journal.revert_all();
        if let BoosterEvent::Restored { restored, failed } = ended {
            rep.restored += restored;
            rep.failed.extend(failed);
            rep.nothing_to_restore = rep.restored == 0 && rep.failed.is_empty();
        }
        rep
    }

    // -- cleaner -------------------------------------------------------------------------

    pub fn cleaner_scan(&self) -> ScanSummary {
        let (scan, summary) = cleaner::scan(&CleanerEnv::from_system(), self.settings().cleaner_min_age_hours as u64);
        *self.scan.lock().unwrap() = Some(scan);
        summary
    }

    pub fn clean(&self, scan_id: &str, selected: &[String]) -> Result<CleanReport> {
        let guard = self.scan.lock().unwrap();
        let scan = guard.as_ref().filter(|s| s.id == scan_id).ok_or_else(|| Error::Invalid("This scan is out of date. Scan again before cleaning.".into()))?;
        let report = cleaner::clean(scan, selected)?;
        if report.total_deleted > 0 || report.total_freed > 0 {
            self.journal.log_irreversible("cleanup", "files", &format!("Cleaned {} ({} files)", crate::analysis::gib(report.total_freed), report.total_deleted), "Permanently deleted. Thomsen Optimizer cannot restore deleted files.");
        }
        *self.temp_cache.lock().unwrap() = None;
        Ok(report)
    }

    // -- startup -------------------------------------------------------------------------

    pub fn startup_list(&self) -> Vec<StartupEntry> {
        startup::list(self.ops.as_ref())
    }

    pub fn startup_set(&self, id: &str, enabled: bool) -> Result<()> {
        let change: PlannedChange = startup::plan_toggle(self.ops.as_ref(), id, enabled)?;
        self.journal.apply("startup", &[change]).map(|_| ()).map_err(|(e, _)| e)
    }

    // -- system info / dashboard ----------------------------------------------------------

    /// Collects static hardware info (slow: one PowerShell query) - call from a
    /// background thread at startup.
    pub fn refresh_system_info(&self) {
        let info = {
            let m = self.monitor.lock().unwrap();
            hwinfo::collect(self.ops.as_ref(), &m.sys)
        };
        *self.sysinfo.lock().unwrap() = Some(info);
    }

    pub fn system_info(&self) -> Option<SystemInfo> {
        self.sysinfo.lock().unwrap().clone()
    }

    pub fn power_plans(&self) -> Result<Vec<PowerPlan>> {
        self.ops.power_plans()
    }

    pub fn set_power_plan(&self, guid: &str) -> Result<()> {
        if !sys::valid_guid(guid) {
            return Err(Error::Invalid("Invalid power plan.".into()));
        }
        let plans = self.ops.power_plans()?;
        let target = plans.iter().find(|p| p.guid.eq_ignore_ascii_case(guid)).ok_or_else(|| Error::NotFound("Power plan not found.".into()))?;
        let change = PlannedChange::PowerPlan { target: target.kind, label: format!("Power plan: {}", target.name), detail: "Changed manually in Advanced settings".into() };
        if target.kind == sys::PlanKind::Custom {
            return Err(Error::Invalid("Custom power plans are managed in Windows settings.".into()));
        }
        self.journal.apply("manual", &[change]).map(|_| ()).map_err(|(e, _)| e)
    }

    pub fn dashboard(&self) -> Dashboard {
        let games: Vec<String> = self.games(false).into_iter().filter(|g| g.running).map(|g| g.name).collect();
        let fps = self.capture.lock().unwrap().as_mut().and_then(|c| c.live().current_fps);
        Dashboard {
            power_plan: self.ops.power_plans().ok().and_then(|p| p.into_iter().find(|x| x.active)),
            booster: self.booster_status(),
            running_games: games,
            restorable_changes: self.journal.active_count(),
            fps,
            elevated: sys::win::is_elevated(),
        }
    }

    // -- FPS -----------------------------------------------------------------------------

    fn presentmon(&self) -> Option<std::path::PathBuf> {
        self.settings().presentmon_path.map(std::path::PathBuf::from).filter(|p| fps::validate_presentmon(p).is_ok()).or_else(|| fps::find_in_tools(self.data.root()))
    }

    pub fn fps_status(&self) -> FpsStatus {
        let pm = self.presentmon();
        FpsStatus { presentmon_ok: pm.is_some(), presentmon_path: pm.map(|p| p.to_string_lossy().into_owned()), capturing: self.capture.lock().unwrap().is_some(), elevated: sys::win::is_elevated() }
    }

    pub fn fps_start(&self, game_id: &str) -> Result<()> {
        if !self.settings().fps_monitoring {
            return Err(Error::Invalid("FPS monitoring is turned off in Settings > Monitoring.".into()));
        }
        let pm = self.presentmon().ok_or_else(|| Error::NotFound("PresentMon is not set up yet.".into()))?;
        let mut cap = self.capture.lock().unwrap();
        if cap.is_some() {
            return Err(Error::Busy("A capture is already running.".into()));
        }
        let game = self.games(false).into_iter().find(|g| g.id == game_id).ok_or_else(|| Error::NotFound("Game not found.".into()))?;
        if !game.running {
            return Err(Error::Invalid(format!("{} is not running. Start the game first.", game.name)));
        }
        // Capture the process that is actually running (its exe name), not a guess.
        let procs_now = self.processes();
        let proc_name = procs_now.iter().find(|g| g.members.iter().any(|m| game.running_pids.contains(&m.pid))).map(|g| g.name.clone()).ok_or_else(|| Error::NotFound("Could not find the game's process.".into()))?;
        *cap = Some(Capture::start(&pm, &proc_name, &game.name)?);
        let active_profile = self.booster_status().profile_id;
        *self.capture_meta.lock().unwrap() = Some((game.name, active_profile));
        Ok(())
    }

    pub fn fps_live(&self) -> LiveFps {
        match self.capture.lock().unwrap().as_mut() {
            Some(c) => c.live(),
            None => LiveFps { capturing: false, game_name: None, frames: 0, elapsed_secs: 0.0, recent: vec![], current_fps: None, waiting_for_frames: false, error: None },
        }
    }

    /// Stops the capture; saves it as a session when `save` is true.
    pub fn fps_stop(&self, save: bool, label: &str) -> Result<Option<FpsSession>> {
        let cap = self.capture.lock().unwrap().take().ok_or_else(|| Error::Invalid("No capture is running.".into()))?;
        let (game, process, started) = (cap.game_name.clone(), cap.process_name.clone(), cap.started_at);
        let frames = cap.finish();
        let meta = self.capture_meta.lock().unwrap().take();
        if !save {
            return Ok(None);
        }
        let retention = self.settings().history_retention_days;
        self.history.add(&game, &process, started, &frames, label, meta.and_then(|m| m.1), retention).map(Some)
    }

    pub fn fps_sessions(&self) -> Vec<FpsSession> {
        self.history.list()
    }

    pub fn fps_compare(&self, a: &str, b: &str) -> Result<Comparison> {
        Ok(compare(self.history.get(a)?, self.history.get(b)?))
    }

    /// Resource use of Thomsen Optimizer itself, for transparency.
    pub fn self_usage(&self) -> (f32, u64) {
        self.latest_sample().map(|s| (s.self_cpu, s.self_memory_bytes)).unwrap_or((0.0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::fake::FakeOps;

    fn engine() -> (tempfile::TempDir, Arc<Engine>) {
        let d = tempfile::tempdir().unwrap();
        let e = Engine::new(DataDir::new(d.path()).unwrap(), Arc::new(FakeOps::with_default_plans()));
        (d, e)
    }

    #[test]
    fn events_serialize_with_adjacent_tags() {
        let v = serde_json::to_value(EngineEvent::Booster(BoosterEvent::Restored { restored: 2, failed: vec![] })).unwrap();
        assert_eq!(v["type"], "booster");
        assert_eq!(v["data"]["type"], "restored");
        assert_eq!(v["data"]["restored"], 2);
    }

    #[test]
    fn cadence_adapts_to_what_is_on_screen() {
        let (_d, e) = engine();
        e.set_ui_state(true, true);
        assert_eq!(e.sample_interval(), Duration::from_millis(1000));
        e.set_ui_state(true, false);
        assert_eq!(e.sample_interval(), Duration::from_secs(5));
        e.set_window_hidden(true);
        assert_eq!(e.sample_interval(), Duration::from_secs(15));
    }

    #[test]
    fn settings_validation_and_persistence() {
        let (d, e) = engine();
        let mut s = e.settings();
        s.refresh_ms = 1;
        s.profiles[0].close_apps = vec!["OneDrive.exe".into(), "..\\evil.exe".into(), "calc".into()];
        s.presentmon_path = Some("C:\\Windows\\notepad.exe".into());
        assert!(e.update_settings(s.clone()).is_err(), "non-PresentMon executables are refused");
        s.presentmon_path = None;
        let saved = e.update_settings(s).unwrap();
        assert_eq!(saved.refresh_ms, 500);
        assert_eq!(saved.profiles[0].close_apps, vec!["OneDrive.exe".to_string()]);
        // Survives a restart.
        let e2 = Engine::new(DataDir::new(d.path()).unwrap(), Arc::new(FakeOps::with_default_plans()));
        assert_eq!(e2.settings().refresh_ms, 500);
    }

    #[test]
    fn restore_all_undoes_applied_changes() {
        let (_d, e) = engine();
        let r = e.activate_profile("competitive", false).unwrap();
        assert!(r.applied >= 1);
        assert!(e.booster_status().active);
        let rep = e.restore_all();
        assert!(rep.restored >= 1);
        assert!(!e.booster_status().active);
        assert_eq!(e.power_plans().unwrap().iter().find(|p| p.active).unwrap().kind, sys::PlanKind::Balanced);
    }

    #[test]
    fn unknown_profile_and_stale_scan_are_rejected() {
        let (_d, e) = engine();
        assert!(e.activate_profile("nope", false).is_err());
        assert!(e.clean("stale", &["user-temp".into()]).is_err());
        assert!(e.startup_set("hkcu-run|ghost", false).is_err());
    }
}
