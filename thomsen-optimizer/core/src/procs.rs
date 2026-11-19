//! Process listing, protection rules and (safe) closing.
//!
//! Hard rules, enforced here rather than in the UI:
//!  * Windows components, security software, anti-cheat services, GPU driver
//!    services, this app and its WebView are **protected** and can never be
//!    closed through this module.
//!  * A process with a visible window gets a polite close request (WM_CLOSE),
//!    so the application can ask about unsaved work. Only background
//!    processes without a window are terminated.
//!  * Targets are re-validated (name + start time) right before acting, so a
//!    recycled PID can never hit the wrong program.

use crate::sys::win;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

const CRITICAL: &[&str] = &[
    "system", "system idle process", "registry", "memory compression", "secure system", "smss.exe", "csrss.exe", "wininit.exe", "services.exe", "lsass.exe", "lsm.exe",
    "winlogon.exe", "svchost.exe", "dwm.exe", "explorer.exe", "fontdrvhost.exe", "sihost.exe", "taskhostw.exe", "ctfmon.exe", "runtimebroker.exe", "audiodg.exe", "spoolsv.exe",
    "conhost.exe", "dllhost.exe", "searchhost.exe", "searchindexer.exe", "startmenuexperiencehost.exe", "shellexperiencehost.exe", "textinputhost.exe", "applicationframehost.exe",
    "wudfhost.exe", "dashost.exe", "taskmgr.exe", "msmpeng.exe", "nissrv.exe", "securityhealthservice.exe", "securityhealthsystray.exe", "smartscreen.exe", "mpdefendercoreservice.exe",
    "nvcontainer.exe", "nvdisplay.container.exe", "amdrsserv.exe", "radeonsoftware.exe", "atieclxx.exe", "atiesrxx.exe", "igfxem.exe", "igfxtray.exe", "msedgewebview2.exe",
];

/// Substrings that mark anti-cheat and security products.
const CRITICAL_FRAGMENTS: &[&str] = &[
    "anticheat", "easyanticheat", "beservice", "battleye", "vgc.exe", "vgtray", "faceit", "esea", "equ8", "pnkbst", "xigncode", "gameguard", "ricochet", "avp.exe", "avgsvc", "avastsvc",
    "bdservicehost", "bdagent", "ekrn.exe", "mcshield", "sophos", "mbamservice", "windefend", "csfalcon", "crowdstrike",
];

/// Launchers that must keep running while their game is running.
const LAUNCHERS: &[&str] = &["steam.exe", "steamwebhelper.exe", "steamservice.exe", "epicgameslauncher.exe", "epicwebhelper.exe", "battle.net.exe", "agent.exe", "eadesktop.exe", "ealauncher.exe", "origin.exe", "upc.exe", "uplaywebcore.exe", "riotclientservices.exe", "riotclientux.exe", "gamingservices.exe", "xboxpcapp.exe"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub cpu: f32,
    pub memory_bytes: u64,
    pub start_time: u64,
    pub has_window: bool,
    pub protected: bool,
    pub protected_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcGroup {
    pub name: String,
    pub exe: Option<String>,
    pub count: usize,
    pub cpu: f32,
    pub memory_bytes: u64,
    pub has_window: bool,
    pub protected: bool,
    pub protected_reason: Option<String>,
    pub members: Vec<ClosePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosePlan {
    pub pid: u32,
    pub name: String,
    pub start_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseResult {
    pub pid: u32,
    pub name: String,
    pub ok: bool,
    /// `closeRequested` (window asked to close; the app may prompt to save),
    /// `terminated` (background process ended) or `skipped`.
    pub method: String,
    pub message: Option<String>,
}

pub struct ProtectContext<'a> {
    pub own_pid: u32,
    pub windows_dir: Option<&'a str>,
    pub game_running: bool,
    /// PIDs descended from this app (its WebView helpers).
    pub own_tree: &'a HashSet<u32>,
}

pub fn protection_reason(name: &str, exe: Option<&str>, pid: u32, ctx: &ProtectContext) -> Option<String> {
    let n = name.to_ascii_lowercase();
    if pid == 0 || pid == 4 {
        return Some("Windows core".into());
    }
    if pid == ctx.own_pid || ctx.own_tree.contains(&pid) {
        return Some("Thomsen Optimizer".into());
    }
    if CRITICAL.contains(&n.as_str()) {
        return Some("Windows or driver component".into());
    }
    if CRITICAL_FRAGMENTS.iter().any(|f| n.contains(f)) {
        return Some("Security or anti-cheat software".into());
    }
    if let (Some(e), Some(w)) = (exe, ctx.windows_dir) {
        if e.to_ascii_lowercase().replace('/', "\\").starts_with(&w.to_ascii_lowercase().replace('/', "\\")) {
            return Some("Part of Windows".into());
        }
    }
    if ctx.game_running && LAUNCHERS.contains(&n.as_str()) {
        return Some("Game launcher (a game is running)".into());
    }
    None
}

fn descendants(sys: &System, root: u32) -> HashSet<u32> {
    let mut set = HashSet::new();
    set.insert(root);
    // A handful of passes is enough for the shallow WebView tree.
    for _ in 0..4 {
        for (pid, p) in sys.processes() {
            if let Some(parent) = p.parent() {
                if set.contains(&parent.as_u32()) {
                    set.insert(pid.as_u32());
                }
            }
        }
    }
    set
}

pub fn refresh(sys: &mut System) {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(UpdateKind::OnlyIfNotSet));
}

pub fn windows_dir() -> Option<String> {
    std::env::var("SystemRoot").or_else(|_| std::env::var("windir")).ok()
}

pub fn list(sys: &System, game_running: bool, with_windows: bool) -> Vec<ProcInfo> {
    let own = std::process::id();
    let tree = descendants(sys, own);
    let wd = windows_dir();
    let windows = if with_windows { win::visible_window_pids() } else { HashSet::new() };
    let cores = sys.cpus().len().max(1) as f32;
    let ctx = ProtectContext { own_pid: own, windows_dir: wd.as_deref(), game_running, own_tree: &tree };
    sys.processes()
        .iter()
        .map(|(pid, p)| {
            let name = p.name().to_string_lossy().into_owned();
            let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
            let reason = protection_reason(&name, exe.as_deref(), pid.as_u32(), &ctx);
            ProcInfo {
                pid: pid.as_u32(),
                name,
                exe,
                cpu: p.cpu_usage() / cores,
                memory_bytes: p.memory(),
                start_time: p.start_time(),
                has_window: windows.contains(&pid.as_u32()),
                protected: reason.is_some(),
                protected_reason: reason,
            }
        })
        .collect()
}

/// Groups processes by executable name (e.g. all `chrome.exe` helpers).
pub fn group(procs: &[ProcInfo]) -> Vec<ProcGroup> {
    let mut map: BTreeMap<String, ProcGroup> = BTreeMap::new();
    for p in procs {
        let g = map.entry(p.name.to_ascii_lowercase()).or_insert_with(|| ProcGroup {
            name: p.name.clone(),
            exe: p.exe.clone(),
            count: 0,
            cpu: 0.0,
            memory_bytes: 0,
            has_window: false,
            protected: false,
            protected_reason: None,
            members: vec![],
        });
        g.count += 1;
        g.cpu += p.cpu;
        g.memory_bytes += p.memory_bytes;
        g.has_window |= p.has_window;
        if p.protected {
            g.protected = true;
            g.protected_reason = p.protected_reason.clone();
        }
        if g.exe.is_none() {
            g.exe = p.exe.clone();
        }
        g.members.push(ClosePlan { pid: p.pid, name: p.name.clone(), start_time: p.start_time });
    }
    let mut v: Vec<_> = map.into_values().filter(|g| g.name.to_ascii_lowercase() != "system idle process").collect();
    v.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));
    v
}

/// Closes the given processes, applying every safety rule again.
pub fn close(sys: &mut System, plans: &[ClosePlan], game_running: bool) -> Vec<CloseResult> {
    refresh(sys);
    let own = std::process::id();
    let tree = descendants(sys, own);
    let wd = windows_dir();
    let windows = win::visible_window_pids();
    let ctx = ProtectContext { own_pid: own, windows_dir: wd.as_deref(), game_running, own_tree: &tree };
    let mut out = Vec::new();
    for plan in plans {
        let skip = |msg: &str| CloseResult { pid: plan.pid, name: plan.name.clone(), ok: false, method: "skipped".into(), message: Some(msg.into()) };
        let Some(p) = sys.process(Pid::from_u32(plan.pid)) else {
            out.push(skip("Already closed."));
            continue;
        };
        let name = p.name().to_string_lossy().into_owned();
        if !name.eq_ignore_ascii_case(&plan.name) || p.start_time() != plan.start_time {
            out.push(skip("The process changed since it was listed."));
            continue;
        }
        let exe = p.exe().map(|e| e.to_string_lossy().into_owned());
        if let Some(r) = protection_reason(&name, exe.as_deref(), plan.pid, &ctx) {
            out.push(skip(&format!("Protected: {r}.")));
            continue;
        }
        if windows.contains(&plan.pid) {
            let n = win::request_close(plan.pid);
            out.push(CloseResult { pid: plan.pid, name, ok: n > 0, method: "closeRequested".into(), message: if n > 0 { Some("Asked to close; the app may ask to save first.".into()) } else { Some("Could not send a close request.".into()) } });
        } else if p.kill() {
            out.push(CloseResult { pid: plan.pid, name, ok: true, method: "terminated".into(), message: None });
        } else {
            out.push(CloseResult { pid: plan.pid, name, ok: false, method: "skipped".into(), message: Some("Windows did not allow closing this process.".into()) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(tree: &'a HashSet<u32>, game: bool) -> ProtectContext<'a> {
        ProtectContext { own_pid: 100, windows_dir: Some("C:\\Windows"), game_running: game, own_tree: tree }
    }

    #[test]
    fn protects_critical_things() {
        let t: HashSet<u32> = [101].into();
        let c = ctx(&t, false);
        assert!(protection_reason("csrss.exe", None, 500, &c).is_some());
        assert!(protection_reason("EasyAntiCheat_EOS.exe", None, 501, &c).is_some());
        assert!(protection_reason("anything.exe", Some("C:\\WINDOWS\\System32\\x.exe"), 502, &c).is_some());
        assert!(protection_reason("me.exe", None, 100, &c).is_some());
        assert!(protection_reason("webview.exe", None, 101, &c).is_some());
        assert!(protection_reason("OneDrive.exe", Some("C:\\Users\\a\\OneDrive.exe"), 503, &c).is_none());
    }

    #[test]
    fn launchers_protected_only_while_gaming() {
        let t = HashSet::new();
        assert!(protection_reason("steam.exe", None, 9, &ctx(&t, false)).is_none());
        assert!(protection_reason("Steam.exe", None, 9, &ctx(&t, true)).is_some());
    }

    #[test]
    fn grouping_sums_and_flags_protection() {
        let mk = |pid, name: &str, mem, prot: bool| ProcInfo { pid, name: name.into(), exe: None, cpu: 1.0, memory_bytes: mem, start_time: 1, has_window: false, protected: prot, protected_reason: prot.then(|| "x".into()) };
        let g = group(&[mk(1, "a.exe", 10, false), mk(2, "A.exe", 20, false), mk(3, "b.exe", 5, true)]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].count, 2);
        assert_eq!(g[0].memory_bytes, 30);
        assert!(g[1].protected);
    }
}
