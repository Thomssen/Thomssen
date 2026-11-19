//! Tauri command layer. Every command validates its input in the core and
//! runs blocking work off the UI thread. Nothing accepts a command line,
//! shell string or arbitrary path to execute.

use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use thomsen_optimizer_core::analysis::Recommendation;
use thomsen_optimizer_core::booster::{BoosterEvent, BoosterStatus};
use thomsen_optimizer_core::cleaner::{CleanReport, ScanSummary};
use thomsen_optimizer_core::engine::{AnalysisReport, Dashboard, FpsStatus, OptimizeRequest};
use thomsen_optimizer_core::fps::LiveFps;
use thomsen_optimizer_core::games::{Game, GameSettings};
use thomsen_optimizer_core::history::{Comparison, FpsSession};
use thomsen_optimizer_core::hwinfo::SystemInfo;
use thomsen_optimizer_core::journal::{ApplyOutcome, ChangeRecord, RestoreReport};
use thomsen_optimizer_core::monitor::Sample;
use thomsen_optimizer_core::optimizer::OptimizationResult;
use thomsen_optimizer_core::procs::{ClosePlan, ProcGroup};
use thomsen_optimizer_core::settings::Settings;
use thomsen_optimizer_core::startup::StartupEntry;
use thomsen_optimizer_core::sys::PowerPlan;
use thomsen_optimizer_core::Error;

type R<T> = Result<T, Error>;

/// Runs `f` on the blocking pool; the engine's calls may touch disk, the
/// registry or helper processes.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> R<T> + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| Error::Command(format!("background task failed: {e}")))?
}

macro_rules! cmd {
    ($name:ident($($arg:ident : $ty:ty),*) -> $ret:ty, |$e:ident| $body:expr) => {
        #[tauri::command]
        pub async fn $name(state: State<'_, AppState>, $($arg: $ty),*) -> R<$ret> {
            let $e = state.engine.clone();
            blocking(move || $body).await
        }
    };
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: &'static str,
    version: String,
    elevated: bool,
    data_dir: String,
    self_cpu: f32,
    self_memory_bytes: u64,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    let (cpu, mem) = state.engine.self_usage();
    AppInfo { name: "Thomsen Optimizer", version: app.package_info().version.to_string(), elevated: thomsen_optimizer_core::sys::win::is_elevated(), data_dir: state.engine.data.root().to_string_lossy().into_owned(), self_cpu: cpu, self_memory_bytes: mem }
}

#[tauri::command]
pub fn set_ui_state(state: State<'_, AppState>, visible: bool, monitoring: bool) {
    state.engine.set_ui_state(visible, monitoring);
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.engine.settings()
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> R<Settings> {
    let engine = state.engine.clone();
    let before = engine.settings();
    let saved = blocking(move || engine.update_settings(settings)).await?;
    if saved.launch_on_startup != before.launch_on_startup {
        let al = app.autolaunch();
        let res = if saved.launch_on_startup { al.enable() } else { al.disable() };
        if let Err(e) = res {
            log::warn!("autostart change failed: {e}");
            return Err(Error::Command("Windows did not allow changing the startup entry.".into()));
        }
    }
    Ok(saved)
}

#[tauri::command]
pub fn autostart_enabled(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

// -- system -------------------------------------------------------------------------------
#[tauri::command]
pub fn system_info(state: State<'_, AppState>) -> Option<SystemInfo> {
    state.engine.system_info()
}
#[tauri::command]
pub fn latest_sample(state: State<'_, AppState>) -> Option<Sample> {
    state.engine.latest_sample()
}
cmd!(dashboard() -> Dashboard, |e| Ok(e.dashboard()));
cmd!(power_plans() -> Vec<PowerPlan>, |e| e.power_plans());
cmd!(set_power_plan(guid: String) -> (), |e| e.set_power_plan(&guid));

// -- optimization ---------------------------------------------------------------------------
cmd!(analyze() -> AnalysisReport, |e| Ok(e.analyze()));
cmd!(optimize_now() -> OptimizationResult, |e| e.optimize_now());
cmd!(optimize_selected(request: OptimizeRequest) -> OptimizationResult, |e| e.optimize_selected(request));
cmd!(apply_recommendation(id: String) -> OptimizationResult, |e| e.apply_recommendation(&id));
cmd!(optimization_results() -> Vec<OptimizationResult>, |e| Ok(e.results()));
cmd!(list_processes() -> Vec<ProcGroup>, |e| Ok(e.processes()));
cmd!(close_processes(plans: Vec<ClosePlan>) -> OptimizationResult, |e| Ok(e.close_processes(plans)));

// -- games & booster ---------------------------------------------------------------------------
cmd!(list_games(rescan: bool) -> Vec<Game>, |e| Ok(e.games(rescan)));
cmd!(add_game(name: String, exe_path: String) -> (), |e| e.add_game(&name, &exe_path));
cmd!(remove_game(id: String) -> (), |e| e.remove_game(&id));
cmd!(update_game(id: String, settings: GameSettings) -> GameSettings, |e| e.update_game(&id, settings));
cmd!(set_gpu_preference(game_id: String, high: bool) -> Vec<ApplyOutcome>, |e| e.set_gpu_preference(&game_id, high));
cmd!(activate_profile(profile_id: String, close_apps: bool) -> OptimizationResult, |e| e.activate_profile(&profile_id, close_apps));
cmd!(deactivate_profile() -> BoosterEvent, |e| Ok(e.deactivate_profile()));
#[tauri::command]
pub fn booster_status(state: State<'_, AppState>) -> BoosterStatus {
    state.engine.booster_status()
}

// -- cleaner / startup -------------------------------------------------------------------------
cmd!(cleaner_scan() -> ScanSummary, |e| Ok(e.cleaner_scan()));
cmd!(cleaner_clean(scan_id: String, categories: Vec<String>) -> CleanReport, |e| e.clean(&scan_id, &categories));
cmd!(startup_list() -> Vec<StartupEntry>, |e| Ok(e.startup_list()));
cmd!(startup_set(id: String, enabled: bool) -> (), |e| e.startup_set(&id, enabled));

// -- restore --------------------------------------------------------------------------------------
cmd!(history_list() -> Vec<ChangeRecord>, |e| Ok(e.history_list()));
cmd!(restore_change(id: String) -> (), |e| e.restore_one(&id));
cmd!(restore_all() -> RestoreReport, |e| Ok(e.restore_all()));
cmd!(create_restore_point() -> (), |e| e.create_restore_point());

// -- FPS ------------------------------------------------------------------------------------------
cmd!(fps_status() -> FpsStatus, |e| Ok(e.fps_status()));
cmd!(fps_start(game_id: String) -> (), |e| e.fps_start(&game_id));
#[tauri::command]
pub fn fps_live(state: State<'_, AppState>) -> LiveFps {
    state.engine.fps_live()
}
cmd!(fps_stop(save: bool, label: String) -> Option<FpsSession>, |e| e.fps_stop(save, &label));
cmd!(fps_sessions() -> Vec<FpsSession>, |e| Ok(e.fps_sessions()));
cmd!(fps_compare(a: String, b: String) -> Comparison, |e| e.fps_compare(&a, &b));
cmd!(fps_update_session(id: String, label: String, notes: String) -> (), |e| e.history.update(&id, &label, &notes));
cmd!(fps_delete_session(id: String) -> (), |e| e.history.delete(&id));
cmd!(fps_clear_sessions() -> (), |e| e.history.clear());

// -- OS integration (fixed allow-lists only) ---------------------------------------------------------------
#[tauri::command]
pub async fn pick_executable(app: AppHandle, title: String) -> R<Option<String>> {
    let t: String = title.chars().take(60).collect();
    blocking(move || {
        let p = app.dialog().file().set_title(t).add_filter("Programs", &["exe"]).blocking_pick_file();
        Ok(p.and_then(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()))
    })
    .await
}

/// Opens one of a fixed set of locations - never a caller-supplied URL.
#[tauri::command]
pub fn open_external(app: AppHandle, state: State<'_, AppState>, target: String) -> R<()> {
    let url = match target.as_str() {
        "presentmon" => "https://github.com/GameTechDev/PresentMon/releases".to_string(),
        "website" => "https://thomsen-software.com".to_string(),
        "github" => "https://github.com/Thomssen/Thomsen-Optimizer".to_string(),
        "releases" => "https://github.com/Thomssen/Thomsen-Optimizer/releases".to_string(),
        "gameMode" => "ms-settings:gaming-gamemode".to_string(),
        "graphics" => "ms-settings:display-advancedgraphics".to_string(),
        "power" => "ms-settings:powersleep".to_string(),
        "dataFolder" => state.engine.data.root().to_string_lossy().into_owned(),
        _ => return Err(Error::Invalid("Unknown link.".into())),
    };
    let r = if target == "dataFolder" { app.opener().open_path(url, None::<&str>) } else { app.opener().open_url(url, None::<&str>) };
    r.map_err(|e| Error::Command(format!("Could not open it: {e}")))
}

/// Relaunches this app with administrator rights (UAC prompt) - used only on
/// request, e.g. so PresentMon can read frame events.
#[tauri::command]
pub fn relaunch_elevated(app: AppHandle, state: State<'_, AppState>) -> R<()> {
    let exe = std::env::current_exe().map_err(|e| Error::Command(e.to_string()))?;
    state.engine.deactivate_profile();
    thomsen_optimizer_core::sys::win::shell_runas(&exe, "")?;
    app.exit(0);
    Ok(())
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        app_info,
        set_ui_state,
        get_settings,
        save_settings,
        autostart_enabled,
        system_info,
        latest_sample,
        dashboard,
        power_plans,
        set_power_plan,
        analyze,
        optimize_now,
        optimize_selected,
        apply_recommendation,
        optimization_results,
        list_processes,
        close_processes,
        list_games,
        add_game,
        remove_game,
        update_game,
        set_gpu_preference,
        activate_profile,
        deactivate_profile,
        booster_status,
        cleaner_scan,
        cleaner_clean,
        startup_list,
        startup_set,
        history_list,
        restore_change,
        restore_all,
        create_restore_point,
        fps_status,
        fps_start,
        fps_live,
        fps_stop,
        fps_sessions,
        fps_compare,
        fps_update_session,
        fps_delete_session,
        fps_clear_sessions,
        pick_executable,
        open_external,
        relaunch_elevated,
        crate::updater::update_check,
        crate::updater::update_install,
    ]
}

#[allow(dead_code)]
fn _types(_: Recommendation) {}
