//! Tiny file logger. Messages are written by our own code and never contain screen text or
//! personal data (the detection pipeline does not keep any); the file is capped at 256 KiB.

use log::{LevelFilter, Log, Metadata, Record};
use parking_lot::Mutex;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

struct FileLogger {
    path: PathBuf,
    file: Mutex<Option<File>>,
}

impl Log for FileLogger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.target().starts_with("privacy_blur")
    }
    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let mut g = self.file.lock();
        if let Some(f) = g.as_mut() {
            if f.metadata().map(|m| m.len() > 256 * 1024).unwrap_or(false) {
                let _ = fs::rename(&self.path, self.path.with_extension("old.log"));
                *g = OpenOptions::new().create(true).append(true).open(&self.path).ok();
            }
        }
        if let Some(f) = g.as_mut() {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let _ = writeln!(f, "{secs} {:<5} {}", r.level(), r.args());
        }
    }
    fn flush(&self) {}
}

pub fn init(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("privacy-blur.log");
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    let logger = Box::leak(Box::new(FileLogger { path, file: Mutex::new(file) }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
