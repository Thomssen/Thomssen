//! Persistence. `settings.json` holds only non-sensitive preferences. The personal data the
//! user types in (names, addresses, …) lives in `entries.dat`, encrypted with Windows DPAPI
//! so it can only be decrypted by the same Windows user on the same machine.

use crate::win::dpapi;
use privacy_core::{Entry, Settings};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Store {
    dir: PathBuf,
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("replace {}: {e}", path.display()))
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn load_settings(&self) -> (Settings, Option<String>) {
        let p = self.dir.join("settings.json");
        match fs::read(&p) {
            Ok(b) => match serde_json::from_slice::<Settings>(&b) {
                Ok(mut s) => {
                    s.sanitize();
                    (s, None)
                }
                Err(_) => {
                    let _ = fs::rename(&p, self.dir.join("settings.json.bad"));
                    (Settings::default(), Some("Settings file was damaged; defaults were restored.".into()))
                }
            },
            Err(_) => (Settings::default(), None),
        }
    }

    pub fn save_settings(&self, s: &Settings) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
        atomic_write(&self.dir.join("settings.json"), &json)
    }

    pub fn load_entries(&self) -> (Vec<Entry>, Option<String>) {
        let p = self.dir.join("entries.dat");
        let Ok(cipher) = fs::read(&p) else { return (vec![], None) };
        match dpapi::unprotect(&cipher).and_then(|plain| serde_json::from_slice::<Vec<Entry>>(&plain).map_err(|e| e.to_string())) {
            Ok(v) => (v, None),
            Err(e) => {
                let _ = fs::rename(&p, self.dir.join("entries.dat.unreadable"));
                (vec![], Some(format!("Your saved personal information could not be decrypted ({e}). It was set aside and not deleted.")))
            }
        }
    }

    pub fn save_entries(&self, v: &[Entry]) -> Result<(), String> {
        let plain = serde_json::to_vec(v).map_err(|e| e.to_string())?;
        let cipher = dpapi::protect(&plain)?;
        atomic_write(&self.dir.join("entries.dat"), &cipher)
    }
}
