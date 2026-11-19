//! The analysis engine: turns a measured [`Snapshot`] of the PC into
//! personalised, explained recommendations. It is a pure function, so every
//! rule is unit-tested. Nothing is recommended "just because": each rule
//! quotes the measurement that triggered it, states the downside, says
//! whether a restart is needed and how to undo the change.

use crate::journal::PlannedChange;
use crate::sys::{Hive, PlanKind, PowerPlan, RegData};
use serde::{Deserialize, Serialize};

pub const GAMEBAR: &str = "Software\\Microsoft\\GameBar";
pub const GAMEDVR: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\GameDVR";
pub const GAMECFG: &str = "System\\GameConfigStore";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeavyApp {
    pub name: String,
    pub memory_bytes: u64,
    pub cpu: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub cpu_usage_avg: f32,
    pub logical_cores: usize,
    pub mem_total: u64,
    pub mem_available: u64,
    pub power_plan: Option<PowerPlan>,
    pub has_high_perf_plan: bool,
    /// `AutoGameModeEnabled`; `None` = value absent (Windows default: on).
    pub game_mode: Option<u32>,
    pub app_capture: Option<u32>,
    pub historical_capture: Option<u32>,
    pub game_dvr_enabled: Option<u32>,
    pub heavy_apps: Vec<HeavyApp>,
    pub startup_enabled: usize,
    pub temp_bytes: u64,
    /// (free, total, kind) of the system drive.
    pub system_drive: Option<(u64, u64, String)>,
    pub gpu_names: Vec<String>,
    pub uptime_secs: u64,
    pub on_battery: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Cpu,
    Memory,
    Windows,
    Storage,
    Background,
    Gpu,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Measured problem worth fixing.
    Attention,
    /// Optional improvement.
    Suggestion,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RecAction {
    /// Windows settings the journal can apply and reverse.
    Changes { changes: Vec<PlannedChange> },
    /// Opens another page (cleaner, startup manager, process list).
    #[serde(rename_all = "camelCase")]
    Navigate { route: String },
    /// Advice only; nothing for the app to change.
    Guidance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub id: String,
    pub category: Category,
    pub severity: Severity,
    pub title: String,
    /// The measurement behind the recommendation.
    pub evidence: String,
    /// What it changes.
    pub changes: String,
    pub why: String,
    pub downsides: String,
    pub restart_required: bool,
    pub how_to_reverse: String,
    pub action: RecAction,
    /// Included in "Optimize Now" (settings only; never closes apps/deletes files).
    pub safe_auto: bool,
}

pub fn gib(b: u64) -> String {
    format!("{:.1} GB", b as f64 / 1_073_741_824.0)
}

pub fn reg_dword(path: &str, name: &str, v: u32, label: &str, detail: &str) -> PlannedChange {
    PlannedChange::Registry { hive: Hive::CurrentUser, path: path.into(), name: name.into(), new: Some(RegData::Dword(v)), label: label.into(), detail: detail.into() }
}

fn power_saver_rec(plan: &PowerPlan, has_high: bool) -> Recommendation {
    Recommendation {
        id: "power-plan".into(),
        category: Category::Cpu,
        severity: Severity::Attention,
        title: "Switch away from the Power saver plan".into(),
        evidence: format!("Active power plan: {}", plan.name),
        changes: if has_high { "Activates the built-in High performance power plan.".into() } else { "Activates the Balanced power plan.".into() },
        why: "Power saver caps CPU clocks to save energy, which can lower and destabilise frame rates in games.".into(),
        downsides: "Higher power use and heat; shorter battery life on laptops.".into(),
        restart_required: false,
        how_to_reverse: "Restore this change from History, or pick another plan in Windows power settings.".into(),
        action: RecAction::Changes { changes: vec![PlannedChange::PowerPlan { target: if has_high { PlanKind::HighPerformance } else { PlanKind::Balanced }, label: if has_high { "Power plan: High performance".into() } else { "Power plan: Balanced".into() }, detail: format!("Was: {}", plan.name) }] },
        safe_auto: true,
    }
}

/// Pure analysis. Order of the output is the order shown to the user.
pub fn analyze(s: &Snapshot) -> Vec<Recommendation> {
    let mut out = Vec::new();

    // -- Power plan ---------------------------------------------------------
    if let Some(plan) = &s.power_plan {
        if plan.kind == PlanKind::PowerSaver && !s.on_battery {
            out.push(power_saver_rec(plan, s.has_high_perf_plan));
        } else if plan.kind == PlanKind::Balanced {
            out.push(Recommendation {
                id: "power-plan-gaming".into(),
                category: Category::Cpu,
                severity: Severity::Info,
                title: "Balanced power plan is active".into(),
                evidence: format!("Active power plan: {}", plan.name),
                changes: "Nothing right now. Gaming profiles can switch to a higher-performance plan only while a game runs, then switch back.".into(),
                why: "Balanced is a sensible everyday choice. Modern CPUs boost quickly under Balanced, so a permanent switch is rarely necessary.".into(),
                downsides: "-".into(),
                restart_required: false,
                how_to_reverse: "Not applicable.".into(),
                action: RecAction::Navigate { route: "game-booster".into() },
                safe_auto: false,
            });
        }
    }

    // -- Game Mode ------------------------------------------------------------
    if s.game_mode == Some(0) {
        out.push(Recommendation {
            id: "game-mode".into(),
            category: Category::Windows,
            severity: Severity::Suggestion,
            title: "Turn on Windows Game Mode".into(),
            evidence: "Game Mode is switched off for your account.".into(),
            changes: "Sets HKCU\\Software\\Microsoft\\GameBar\\AutoGameModeEnabled to 1.".into(),
            why: "Game Mode asks Windows to prioritise the running game and to hold back Windows Update driver installs and restart notifications while you play.".into(),
            downsides: "Effects are small and game-dependent; a few older titles have reported stutter with Game Mode, in which case it can simply be turned off again.".into(),
            restart_required: false,
            how_to_reverse: "Restore this change from History, or turn Game Mode off in Windows Settings > Gaming.".into(),
            action: RecAction::Changes { changes: vec![reg_dword(GAMEBAR, "AutoGameModeEnabled", 1, "Windows Game Mode: on", "Was: off")] },
            safe_auto: true,
        });
    }

    // -- Game DVR / Game Bar background recording ----------------------------------
    let capturing = s.app_capture == Some(1) || s.historical_capture == Some(1);
    if capturing {
        let mut ch = Vec::new();
        if s.app_capture == Some(1) {
            ch.push(reg_dword(GAMEDVR, "AppCaptureEnabled", 0, "Game Bar capture: off", "Was: on"));
        }
        if s.historical_capture == Some(1) {
            ch.push(reg_dword(GAMEDVR, "HistoricalCaptureEnabled", 0, "Game Bar background recording: off", "Was: on"));
        }
        out.push(Recommendation {
            id: "game-dvr".into(),
            category: Category::Windows,
            severity: Severity::Suggestion,
            title: "Turn off Xbox Game Bar background recording".into(),
            evidence: "Game Bar capture / background recording is enabled.".into(),
            changes: "Sets AppCaptureEnabled / HistoricalCaptureEnabled under HKCU\\...\\GameDVR to 0.".into(),
            why: "Background recording continuously encodes video while you play, which uses GPU encoder time, memory and disk bandwidth.".into(),
            downsides: "You lose \"record what just happened\" clips and Game Bar screenshots/recordings until you re-enable it.".into(),
            restart_required: false,
            how_to_reverse: "Restore this change from History, or enable capture in Windows Settings > Gaming > Captures.".into(),
            action: RecAction::Changes { changes: ch },
            safe_auto: true,
        });
    }

    // -- Memory ---------------------------------------------------------------
    if s.mem_total > 0 {
        let avail_pct = s.mem_available as f64 / s.mem_total as f64 * 100.0;
        let heavy_total: u64 = s.heavy_apps.iter().take(5).map(|a| a.memory_bytes).sum();
        if (avail_pct < 20.0 || s.mem_available < 3 * 1_073_741_824) && !s.heavy_apps.is_empty() {
            let names: Vec<String> = s.heavy_apps.iter().take(3).map(|a| format!("{} ({})", a.name, gib(a.memory_bytes))).collect();
            out.push(Recommendation {
                id: "low-memory".into(),
                category: Category::Memory,
                severity: if avail_pct < 10.0 { Severity::Attention } else { Severity::Suggestion },
                title: "Free memory by closing optional apps".into(),
                evidence: format!("{} of {} RAM available ({avail_pct:.0}%). Largest apps: {}.", gib(s.mem_available), gib(s.mem_total), names.join(", ")),
                changes: "Nothing automatically. You choose which optional applications to close on the Optimize page.".into(),
                why: format!("When RAM runs low Windows pages memory to disk, which causes stutter. The five largest user apps hold about {}.", gib(heavy_total)),
                downsides: "Closing an app ends whatever it was doing. Apps with open windows are asked to close so they can offer to save.".into(),
                restart_required: false,
                how_to_reverse: "Closed apps cannot be restored by Thomsen Optimizer; just start them again.".into(),
                action: RecAction::Navigate { route: "optimize".into() },
                safe_auto: false,
            });
        }
    }

    // -- CPU ------------------------------------------------------------------
    if s.cpu_usage_avg > 35.0 && !s.heavy_apps.is_empty() {
        let hot: Vec<&HeavyApp> = s.heavy_apps.iter().filter(|a| a.cpu > 3.0).take(3).collect();
        if !hot.is_empty() {
            let names: Vec<String> = hot.iter().map(|a| format!("{} ({:.0}%)", a.name, a.cpu)).collect();
            out.push(Recommendation {
                id: "cpu-background".into(),
                category: Category::Background,
                severity: Severity::Attention,
                title: "Background programs are using CPU".into(),
                evidence: format!("CPU is {:.0}% busy on average while idle-ish. Busiest: {}.", s.cpu_usage_avg, names.join(", ")),
                changes: "Nothing automatically; review them on the Optimize page.".into(),
                why: "CPU time used by background programs is not available to your game, which can lower 1% low FPS.".into(),
                downsides: "Some of these may be tools you rely on (sync, backup, streaming).".into(),
                restart_required: false,
                how_to_reverse: "Closed apps can simply be started again.".into(),
                action: RecAction::Navigate { route: "optimize".into() },
                safe_auto: false,
            });
        }
    }

    // -- Startup ---------------------------------------------------------------
    if s.startup_enabled >= 10 {
        out.push(Recommendation {
            id: "startup-many".into(),
            category: Category::Background,
            severity: Severity::Suggestion,
            title: "Many programs start with Windows".into(),
            evidence: format!("{} startup entries are enabled.", s.startup_enabled),
            changes: "Nothing automatically. You can disable individual entries in the Startup Manager (they are not deleted).".into(),
            why: "Every startup program can keep running in the background and use memory and CPU during games.".into(),
            downsides: "Disabled programs will not start on their own - e.g. cloud sync or hardware utilities.".into(),
            restart_required: false,
            how_to_reverse: "Re-enable the entry in the Startup Manager; its original setting is kept.".into(),
            action: RecAction::Navigate { route: "startup".into() },
            safe_auto: false,
        });
    }

    // -- Storage ---------------------------------------------------------------
    if s.temp_bytes >= 500 * 1024 * 1024 {
        out.push(Recommendation {
            id: "temp-files".into(),
            category: Category::Storage,
            severity: Severity::Suggestion,
            title: "Remove temporary files".into(),
            evidence: format!("About {} of old temporary files found in your user temp folder.", gib(s.temp_bytes)),
            changes: "Nothing automatically. The System Cleaner shows every file first and asks for confirmation.".into(),
            why: "Frees disk space. It does not by itself make games run faster.".into(),
            downsides: "Deleted files are permanently removed (not moved to the Recycle Bin).".into(),
            restart_required: false,
            how_to_reverse: "Deleted files cannot be restored by Thomsen Optimizer.".into(),
            action: RecAction::Navigate { route: "cleaner".into() },
            safe_auto: false,
        });
    }
    if let Some((free, total, kind)) = &s.system_drive {
        let pct = *free as f64 / (*total).max(1) as f64 * 100.0;
        if pct < 10.0 || *free < 15 * 1_073_741_824 {
            out.push(Recommendation {
                id: "low-disk".into(),
                category: Category::Storage,
                severity: Severity::Attention,
                title: "System drive is nearly full".into(),
                evidence: format!("{} free of {} ({pct:.0}%) on the Windows drive ({kind}).", gib(*free), gib(*total)),
                changes: "Nothing automatically.".into(),
                why: "Windows needs free space for the page file, updates and shader caches; very full drives (especially SSDs) can slow down and stutter.".into(),
                downsides: "-".into(),
                restart_required: false,
                how_to_reverse: "Not applicable.".into(),
                action: RecAction::Navigate { route: "cleaner".into() },
                safe_auto: false,
            });
        }
        if kind == "HDD" {
            out.push(Recommendation {
                id: "system-hdd".into(),
                category: Category::Storage,
                severity: Severity::Info,
                title: "Windows is installed on a hard disk".into(),
                evidence: "The system drive was detected as a spinning hard disk.".into(),
                changes: "Nothing - this is advice.".into(),
                why: "Moving Windows and your most-played games to an SSD usually reduces load times and hitching far more than any software setting.".into(),
                downsides: "-".into(),
                restart_required: false,
                how_to_reverse: "Not applicable.".into(),
                action: RecAction::Guidance,
                safe_auto: false,
            });
        }
    }

    // -- GPU -------------------------------------------------------------------
    if s.gpu_names.len() >= 2 {
        out.push(Recommendation {
            id: "gpu-preference".into(),
            category: Category::Gpu,
            severity: Severity::Suggestion,
            title: "Make sure games use your dedicated GPU".into(),
            evidence: format!("{} graphics adapters found: {}.", s.gpu_names.len(), s.gpu_names.join(", ")),
            changes: "On the Games page you can set Windows' per-app graphics preference to High performance for each game.".into(),
            why: "On PCs with integrated + dedicated graphics Windows occasionally runs a game on the weaker integrated GPU.".into(),
            downsides: "On laptops the dedicated GPU uses more power.".into(),
            restart_required: false,
            how_to_reverse: "Restore the change from History or set the app back to \"Let Windows decide\" in Settings > System > Display > Graphics.".into(),
            action: RecAction::Navigate { route: "games".into() },
            safe_auto: false,
        });
    }
    if !s.gpu_names.is_empty() {
        out.push(Recommendation {
            id: "hags".into(),
            category: Category::Gpu,
            severity: Severity::Info,
            title: "Hardware-accelerated GPU scheduling (guidance)".into(),
            evidence: "Available on supported GPUs/drivers (Windows Settings > System > Display > Graphics).".into(),
            changes: "Nothing - this setting is deliberately not changed by the app.".into(),
            why: "Results vary: it helps some systems (notably with frame generation) and causes stutter on others. Test it yourself with the Performance page.".into(),
            downsides: "Requires a restart to change; not all drivers behave well.".into(),
            restart_required: true,
            how_to_reverse: "Toggle it back in Windows Settings.".into(),
            action: RecAction::Guidance,
            safe_auto: false,
        });
    }

    // -- Uptime ----------------------------------------------------------------
    if s.uptime_secs > 14 * 86_400 {
        out.push(Recommendation {
            id: "uptime".into(),
            category: Category::Windows,
            severity: Severity::Info,
            title: "Restart Windows".into(),
            evidence: format!("This PC has been running for {} days.", s.uptime_secs / 86_400),
            changes: "Nothing - restart when convenient.".into(),
            why: "A restart clears leaked memory and applies pending updates and driver changes.".into(),
            downsides: "-".into(),
            restart_required: true,
            how_to_reverse: "Not applicable.".into(),
            action: RecAction::Guidance,
            safe_auto: false,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::{GUID_BALANCED, GUID_SAVER};

    fn plan(kind: PlanKind, guid: &str) -> PowerPlan {
        PowerPlan { guid: guid.into(), name: "x".into(), active: true, kind }
    }
    fn ids(r: &[Recommendation]) -> Vec<&str> {
        r.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn healthy_system_gets_no_fixes() {
        let s = Snapshot { mem_total: 32 << 30, mem_available: 20 << 30, power_plan: Some(plan(PlanKind::Balanced, GUID_BALANCED)), ..Default::default() };
        let r = analyze(&s);
        assert!(r.iter().all(|r| r.severity == Severity::Info), "{:?}", ids(&r));
        assert!(!r.iter().any(|r| r.safe_auto));
    }

    #[test]
    fn power_saver_plugged_in_is_flagged_but_not_on_battery() {
        let mut s = Snapshot { power_plan: Some(plan(PlanKind::PowerSaver, GUID_SAVER)), has_high_perf_plan: true, ..Default::default() };
        assert!(ids(&analyze(&s)).contains(&"power-plan"));
        s.on_battery = true;
        assert!(!ids(&analyze(&s)).contains(&"power-plan"));
    }

    #[test]
    fn game_mode_only_flagged_when_explicitly_off() {
        let mut s = Snapshot::default();
        assert!(!ids(&analyze(&s)).contains(&"game-mode"));
        s.game_mode = Some(0);
        assert!(ids(&analyze(&s)).contains(&"game-mode"));
        s.game_mode = Some(1);
        assert!(!ids(&analyze(&s)).contains(&"game-mode"));
    }

    #[test]
    fn game_dvr_only_changes_what_is_enabled() {
        let s = Snapshot { app_capture: Some(0), historical_capture: Some(1), ..Default::default() };
        let r = analyze(&s);
        let rec = r.iter().find(|r| r.id == "game-dvr").unwrap();
        match &rec.action {
            RecAction::Changes { changes } => assert_eq!(changes.len(), 1),
            _ => panic!(),
        }
    }

    #[test]
    fn low_memory_cites_measurements_and_never_auto_applies() {
        let s = Snapshot { mem_total: 16 << 30, mem_available: 1 << 30, heavy_apps: vec![HeavyApp { name: "chrome.exe".into(), memory_bytes: 4 << 30, cpu: 1.0 }], ..Default::default() };
        let r = analyze(&s);
        let rec = r.iter().find(|r| r.id == "low-memory").unwrap();
        assert!(rec.evidence.contains("chrome.exe") && !rec.safe_auto);
        assert_eq!(rec.severity, Severity::Attention);
    }

    #[test]
    fn storage_rules() {
        let s = Snapshot { temp_bytes: 2 << 30, system_drive: Some((5 << 30, 500 << 30, "HDD".into())), ..Default::default() };
        let r = analyze(&s);
        let i = ids(&r);
        assert!(i.contains(&"temp-files") && i.contains(&"low-disk") && i.contains(&"system-hdd"));
    }

    #[test]
    fn every_recommendation_is_fully_explained() {
        let s = Snapshot {
            cpu_usage_avg: 60.0,
            mem_total: 16 << 30,
            mem_available: 1 << 30,
            heavy_apps: vec![HeavyApp { name: "a.exe".into(), memory_bytes: 3 << 30, cpu: 20.0 }],
            power_plan: Some(plan(PlanKind::PowerSaver, GUID_SAVER)),
            game_mode: Some(0),
            app_capture: Some(1),
            startup_enabled: 20,
            temp_bytes: 3 << 30,
            system_drive: Some((1 << 30, 100 << 30, "HDD".into())),
            gpu_names: vec!["Intel UHD".into(), "NVIDIA RTX".into()],
            uptime_secs: 30 * 86_400,
            ..Default::default()
        };
        let r = analyze(&s);
        assert!(r.len() >= 10);
        for rec in r {
            assert!(!rec.title.is_empty() && !rec.evidence.is_empty() && !rec.changes.is_empty() && !rec.why.is_empty() && !rec.downsides.is_empty() && !rec.how_to_reverse.is_empty(), "{}", rec.id);
        }
    }
}
