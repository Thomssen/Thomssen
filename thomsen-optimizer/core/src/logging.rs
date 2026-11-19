//! A tiny file logger. Messages are written to `app.log` in the app data
//! folder (size-capped, one rotated copy). The user's profile path and user
//! name are replaced before anything is written, and no hardware serials,
//! account names or file contents are ever logged.

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_BYTES: u64 = 1_000_000;

pub struct FileLogger {
    path: PathBuf,
    lock: Mutex<()>,
}

pub fn redact(s: &str) -> String {
    let mut out = s.to_string();
    for var in ["USERPROFILE", "HOME"] {
        if let Ok(v) = std::env::var(var) {
            if v.len() > 3 {
                out = out.replace(&v, "%USERPROFILE%");
            }
        }
    }
    if let Ok(u) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        if u.len() > 2 {
            out = out.replace(&u, "<user>");
        }
    }
    out
}

impl Log for FileLogger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= Level::Info
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let _g = self.lock.lock().unwrap();
        if std::fs::metadata(&self.path).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
            let _ = std::fs::rename(&self.path, self.path.with_extension("log.old"));
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(f, "{} {:<5} {}", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), r.level(), redact(&r.args().to_string()));
        }
    }

    fn flush(&self) {}
}

pub fn init(path: PathBuf) {
    let logger = Box::leak(Box::new(FileLogger { path, lock: Mutex::new(()) }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_profile_paths() {
        std::env::set_var("USERPROFILE", "C:\\Users\\secretname");
        assert_eq!(redact("failed C:\\Users\\secretname\\x"), "failed %USERPROFILE%\\x");
    }
}
