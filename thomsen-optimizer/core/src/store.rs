//! Local JSON storage in the per-user app data directory. Writes are atomic
//! (temp file + rename) so a crash mid-write never leaves a half-written
//! settings or journal file. Nothing here is ever sent anywhere.

use crate::error::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Reads a JSON file; a missing file yields `T::default()`. A *corrupt*
    /// file is moved aside (`name.corrupt`) and reported via the log rather
    /// than silently overwritten, so nothing is lost without a trace.
    pub fn read_json<T: DeserializeOwned + Default>(&self, name: &str) -> T {
        let p = self.path(name);
        match std::fs::read(&p) {
            Ok(bytes) => match serde_json::from_slice::<T>(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("{name} could not be parsed ({e}); keeping a copy and starting fresh");
                    let _ = std::fs::rename(&p, self.path(&format!("{name}.corrupt")));
                    T::default()
                }
            },
            Err(_) => T::default(),
        }
    }

    pub fn write_json<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
        let p = self.path(name);
        let tmp = self.path(&format!("{name}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
        std::fs::rename(&tmp, &p)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_corrupt_recovery() {
        let d = tempfile::tempdir().unwrap();
        let s = DataDir::new(d.path()).unwrap();
        s.write_json("a.json", &vec![1, 2, 3]).unwrap();
        assert_eq!(s.read_json::<Vec<i32>>("a.json"), vec![1, 2, 3]);
        assert!(s.read_json::<Vec<i32>>("missing.json").is_empty());
        std::fs::write(s.path("b.json"), b"{not json").unwrap();
        assert!(s.read_json::<Vec<i32>>("b.json").is_empty());
        assert!(s.path("b.json.corrupt").exists());
    }
}
