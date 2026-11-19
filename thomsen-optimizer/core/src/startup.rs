//! Startup Manager. Disabling uses Windows' own mechanism - the
//! `StartupApproved` registry keys that Task Manager writes - so nothing is
//! deleted: the original `Run` value or shortcut stays exactly where it was
//! and the entry can be switched back on at any time. Each toggle goes
//! through the restore journal, so the previous state is recorded.

use crate::error::{Error, Result};
use crate::journal::PlannedChange;
use crate::sys::{win, Hive, RegData, SystemOps};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_HKLM: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_HKLM_32: &str = "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
const APPROVED: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    HkcuRun,
    HklmRun,
    HklmRun32,
    FolderUser,
    FolderCommon,
}

impl Source {
    fn label(self) -> &'static str {
        match self {
            Source::HkcuRun => "Registry - Current user (Run)",
            Source::HklmRun => "Registry - All users (Run)",
            Source::HklmRun32 => "Registry - All users, 32-bit (Run)",
            Source::FolderUser => "Startup folder - Current user",
            Source::FolderCommon => "Startup folder - All users",
        }
    }
    fn approved_key(self) -> &'static str {
        match self {
            Source::HkcuRun | Source::HklmRun => "Run",
            Source::HklmRun32 => "Run32",
            Source::FolderUser | Source::FolderCommon => "StartupFolder",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Source::HkcuRun => "hkcu-run",
            Source::HklmRun => "hklm-run",
            Source::HklmRun32 => "hklm-run32",
            Source::FolderUser => "folder-user",
            Source::FolderCommon => "folder-common",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupEntry {
    pub id: String,
    pub name: String,
    pub command: String,
    pub exe_path: Option<String>,
    pub publisher: Option<String>,
    pub location: String,
    pub source: Source,
    pub enabled: bool,
    pub protected: bool,
    pub protected_reason: Option<String>,
    /// Windows does not expose a reliable per-entry boot-impact figure to
    /// normal apps, so this is always `None` rather than an invented number.
    pub impact: Option<String>,
}

pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => match std::env::var(&after[..j]) {
                Ok(v) => {
                    out.push_str(&v);
                    rest = &after[j + 1..];
                }
                Err(_) => {
                    out.push('%');
                    rest = after;
                }
            },
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Extracts the executable path from a Run-key command line.
pub fn exe_from_command(cmd: &str) -> Option<String> {
    let c = expand_env(cmd.trim());
    if let Some(rest) = c.strip_prefix('"') {
        return rest.find('"').map(|i| rest[..i].to_string());
    }
    let lower = c.to_ascii_lowercase();
    for ext in [".exe", ".bat", ".cmd", ".lnk", ".com"] {
        if let Some(i) = lower.find(ext) {
            return Some(c[..i + ext.len()].to_string());
        }
    }
    c.split_whitespace().next().map(String::from)
}

const PROTECTED_NAMES: &[&str] = &["securityhealth", "windows defender", "windowsdefender", "msmpeng", "windows security", "windowssecurity"];

pub fn protection(name: &str, exe: Option<&str>, windows_dir: Option<&str>) -> Option<String> {
    let n = name.to_ascii_lowercase();
    if PROTECTED_NAMES.iter().any(|p| n.contains(p)) {
        return Some("Windows security component".into());
    }
    if let (Some(e), Some(w)) = (exe, windows_dir) {
        let e = e.to_ascii_lowercase().replace('/', "\\");
        let w = w.to_ascii_lowercase().replace('/', "\\");
        if e.starts_with(&format!("{w}\\system32")) || e.starts_with(&format!("{w}\\syswow64")) {
            return Some("Part of Windows".into());
        }
    }
    None
}

fn is_disabled(data: Option<&RegData>) -> bool {
    matches!(data, Some(RegData::Binary(b)) if !b.is_empty() && b[0] & 1 == 1)
}

fn approved_state(ops: &dyn SystemOps, source: Source, name: &str) -> bool {
    let path = format!("{APPROVED}\\{}", source.approved_key());
    let user = ops.reg_get(Hive::CurrentUser, &path, name).ok().flatten();
    let machine = if matches!(source, Source::HklmRun | Source::HklmRun32 | Source::FolderCommon) { ops.reg_get(Hive::LocalMachine, &path, name).ok().flatten() } else { None };
    !(is_disabled(user.as_ref()) || is_disabled(machine.as_ref()))
}

fn startup_dirs() -> Vec<(Source, PathBuf)> {
    let mut v = Vec::new();
    if let Some(a) = std::env::var_os("APPDATA") {
        v.push((Source::FolderUser, PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")));
    }
    if let Some(a) = std::env::var_os("ProgramData") {
        v.push((Source::FolderCommon, PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs\\StartUp")));
    }
    v
}

fn make(ops: &dyn SystemOps, source: Source, name: String, command: String, exe: Option<String>, win_dir: Option<&str>) -> StartupEntry {
    let publisher = exe.as_deref().and_then(|e| win::company_name(Path::new(e)));
    let reason = protection(&name, exe.as_deref(), win_dir);
    StartupEntry {
        id: format!("{}|{}", source.key(), name),
        enabled: approved_state(ops, source, &name),
        name,
        command,
        exe_path: exe,
        publisher,
        location: source.label().into(),
        source,
        protected: reason.is_some(),
        protected_reason: reason,
        impact: None,
    }
}

pub fn list(ops: &dyn SystemOps) -> Vec<StartupEntry> {
    let wd = crate::procs::windows_dir();
    let mut out = Vec::new();
    for (source, hive, path) in [(Source::HkcuRun, Hive::CurrentUser, RUN), (Source::HklmRun, Hive::LocalMachine, RUN_HKLM), (Source::HklmRun32, Hive::LocalMachine, RUN_HKLM_32)] {
        for (name, data) in ops.reg_values(hive, path).unwrap_or_default() {
            if let Some(cmd) = data.as_str() {
                out.push(make(ops, source, name, cmd.to_string(), exe_from_command(cmd), wd.as_deref()));
            }
        }
    }
    for (source, dir) in startup_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().into_owned();
            if fname.eq_ignore_ascii_case("desktop.ini") || !e.path().is_file() {
                continue;
            }
            let path = e.path().to_string_lossy().into_owned();
            out.push(make(ops, source, fname, path.clone(), Some(path), wd.as_deref()));
        }
    }
    out.sort_by(|a, b| b.enabled.cmp(&a.enabled).then(a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase())));
    out
}

pub fn enabled_count(ops: &dyn SystemOps) -> usize {
    list(ops).iter().filter(|e| e.enabled).count()
}

fn filetime_now() -> u64 {
    // 100-ns intervals since 1601-01-01.
    let unix = chrono::Utc::now().timestamp().max(0) as u64;
    (unix + 11_644_473_600) * 10_000_000
}

/// Builds the journalled change that enables/disables an entry. The entry is
/// looked up in a fresh listing - the caller's id is never trusted blindly.
pub fn plan_toggle(ops: &dyn SystemOps, id: &str, enable: bool) -> Result<PlannedChange> {
    let entry = list(ops).into_iter().find(|e| e.id == id).ok_or_else(|| Error::NotFound("That startup entry no longer exists.".into()))?;
    if entry.protected && !enable {
        return Err(Error::Invalid(format!("{} is a protected Windows component and cannot be disabled here.", entry.name)));
    }
    let mut bytes = vec![0u8; 12];
    if enable {
        bytes[0] = 0x02;
    } else {
        bytes[0] = 0x03;
        bytes[4..12].copy_from_slice(&filetime_now().to_le_bytes());
    }
    Ok(PlannedChange::Registry {
        hive: Hive::CurrentUser,
        path: format!("{APPROVED}\\{}", entry.source.approved_key()),
        name: entry.name.clone(),
        new: Some(RegData::Binary(bytes)),
        label: format!("Startup: {} {}", entry.name, if enable { "enabled" } else { "disabled" }),
        detail: format!("{} - the original entry is kept; re-enable it any time.", entry.location),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::fake::FakeOps;

    #[test]
    fn command_parsing() {
        assert_eq!(exe_from_command("\"C:\\Program Files\\App\\app.exe\" --silent"), Some("C:\\Program Files\\App\\app.exe".into()));
        assert_eq!(exe_from_command("C:\\Tools\\thing.exe /min"), Some("C:\\Tools\\thing.exe".into()));
        assert_eq!(exe_from_command("C:\\Program Files\\App\\app.exe -x"), Some("C:\\Program Files\\App\\app.exe".into()));
    }

    #[test]
    fn env_expansion() {
        std::env::set_var("THOMSEN_TEST_VAR", "X");
        assert_eq!(expand_env("%THOMSEN_TEST_VAR%\\a"), "X\\a");
        assert_eq!(expand_env("100%"), "100%");
        assert_eq!(expand_env("%NOPE_NOT_SET%\\a"), "%NOPE_NOT_SET%\\a");
    }

    #[test]
    fn protection_rules() {
        assert!(protection("SecurityHealth", None, None).is_some());
        assert!(protection("x", Some("C:\\Windows\\System32\\a.exe"), Some("C:\\Windows")).is_some());
        assert!(protection("OneDrive", Some("C:\\Users\\a\\OneDrive.exe"), Some("C:\\Windows")).is_none());
    }

    #[test]
    fn disabled_bit_detection() {
        assert!(is_disabled(Some(&RegData::Binary(vec![3, 0, 0, 0]))));
        assert!(is_disabled(Some(&RegData::Binary(vec![7, 0, 0, 0]))));
        assert!(!is_disabled(Some(&RegData::Binary(vec![2, 0, 0, 0]))));
        assert!(!is_disabled(None));
    }

    #[test]
    fn list_and_toggle_roundtrip_keeps_original_entry() {
        let ops = FakeOps::default();
        ops.reg_set(Hive::CurrentUser, RUN, "Discord", &RegData::Sz("\"C:\\Discord\\Update.exe\" --start".into())).unwrap();
        ops.reg_set(Hive::CurrentUser, RUN, "SecurityHealth", &RegData::Sz("C:\\x.exe".into())).unwrap();
        let l = list(&ops);
        assert_eq!(l.len(), 2);
        assert!(l.iter().all(|e| e.enabled && e.impact.is_none()));
        let id = l.iter().find(|e| e.name == "Discord").unwrap().id.clone();

        let PlannedChange::Registry { hive, path, name, new, .. } = plan_toggle(&ops, &id, false).unwrap() else { panic!() };
        ops.reg_set(hive, &path, &name, new.as_ref().unwrap()).unwrap();
        assert!(!list(&ops).iter().find(|e| e.name == "Discord").unwrap().enabled);
        // The Run value itself is untouched - nothing was deleted.
        assert!(ops.reg_get(Hive::CurrentUser, RUN, "Discord").unwrap().is_some());

        // Protected and unknown entries are refused.
        let sec = l.iter().find(|e| e.name == "SecurityHealth").unwrap().id.clone();
        assert!(plan_toggle(&ops, &sec, false).is_err());
        assert!(plan_toggle(&ops, "hkcu-run|Nope", false).is_err());
    }
}
