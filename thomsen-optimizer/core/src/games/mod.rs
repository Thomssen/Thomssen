//! Game library: detection from launchers, manual additions, per-game
//! settings and running-state matching.
//!
//! Detection is read-only: launcher manifests and install folders are read
//! from disk / the registry. Running state comes from the process list (the
//! same information Task Manager shows). No process memory is opened and
//! nothing is injected, so anti-cheat systems are never involved.

pub mod launchers;
pub mod vdf;

use crate::error::{Error, Result};
use crate::store::DataDir;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const FILE: &str = "games.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Launcher {
    Steam,
    Epic,
    Xbox,
    BattleNet,
    Custom,
}

/// What launcher detection finds (never persisted as-is).
#[derive(Debug, Clone)]
pub struct Detected {
    pub name: String,
    pub launcher: Launcher,
    /// Best guess of the main executable.
    pub exe_path: Option<String>,
    pub install_dir: Option<String>,
    pub last_played: Option<DateTime<Utc>>,
}

/// Per-game settings the user controls; persisted in `games.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameSettings {
    pub profile_id: Option<String>,
    pub auto_boost: bool,
    pub background_management: bool,
    pub memory_monitoring: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    pub exe_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GameStore {
    pub settings: HashMap<String, GameSettings>,
    pub custom: Vec<CustomGame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    pub launcher: Launcher,
    pub exe_path: Option<String>,
    pub install_dir: Option<String>,
    pub last_played: Option<DateTime<Utc>>,
    pub settings: GameSettings,
    pub running: bool,
    pub running_pids: Vec<u32>,
    /// Working-set memory of the game's processes while running.
    pub memory_bytes: u64,
}

pub fn game_id(launcher: Launcher, key: &str) -> String {
    let l = match launcher {
        Launcher::Steam => "steam",
        Launcher::Epic => "epic",
        Launcher::Xbox => "xbox",
        Launcher::BattleNet => "battlenet",
        Launcher::Custom => "custom",
    };
    // FNV-1a over the normalised key: short, stable, filesystem-independent.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.to_ascii_lowercase().replace('/', "\\").bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{l}-{h:016x}")
}

/// Executables that live inside a game's folder but are not the game.
pub fn is_auxiliary_exe(file_name: &str) -> bool {
    let n = file_name.to_ascii_lowercase();
    const FRAGMENTS: &[&str] = &[
        "unins", "uninstall", "crash", "redist", "vcredist", "vc_redist", "dxsetup", "dotnet", "setup", "installer", "helper", "easyanticheat", "beservice", "battleye", "eac_", "anticheat", "launcherhelper", "updater", "report", "cefprocess", "webhelper", "ue4prereq", "prereq", "benchmark", "cleanup", "diagnos", "7z", "ndp", "oalinst", "physx",
    ];
    FRAGMENTS.iter().any(|f| n.contains(f))
}

fn same_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\").trim_end_matches('\\').eq_ignore_ascii_case(b.replace('/', "\\").trim_end_matches('\\'))
}

/// A process belongs to a game if it runs from the game's folder (and is not
/// a helper/anti-cheat service) or *is* the game's executable.
pub fn process_matches(game: &Game, proc_exe: Option<&str>, proc_name: &str) -> bool {
    if is_auxiliary_exe(proc_name) {
        return false;
    }
    if let Some(exe) = proc_exe {
        if let Some(g) = &game.exe_path {
            if same_path(g, exe) {
                return true;
            }
        }
        if let Some(dir) = &game.install_dir {
            let d = dir.replace('/', "\\").trim_end_matches('\\').to_ascii_lowercase() + "\\";
            if exe.replace('/', "\\").to_ascii_lowercase().starts_with(&d) {
                return true;
            }
        }
        return false;
    }
    // Path unreadable (some protected processes): fall back to the exe name
    // - only for games whose executable we know.
    game.exe_path.as_deref().and_then(|p| Path::new(&p.replace('\\', "/")).file_name().map(|f| f.to_string_lossy().into_owned())).is_some_and(|f| f.eq_ignore_ascii_case(proc_name))
}

pub struct Library {
    data: DataDir,
}

impl Library {
    pub fn new(data: DataDir) -> Self {
        Self { data }
    }

    pub fn store(&self) -> GameStore {
        self.data.read_json(FILE)
    }

    fn save(&self, s: &GameStore) -> Result<()> {
        self.data.write_json(FILE, s)
    }

    /// Detected + custom games with their saved settings (running state is
    /// filled in by the caller from the process list).
    pub fn build(&self, detected: Vec<Detected>) -> Vec<Game> {
        let store = self.store();
        let mut out: Vec<Game> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for d in detected {
            let key = d.install_dir.clone().or_else(|| d.exe_path.clone()).unwrap_or_else(|| d.name.clone());
            let id = game_id(d.launcher, &key);
            if !seen.insert(id.clone()) {
                continue;
            }
            let settings = store.settings.get(&id).cloned().unwrap_or_default();
            out.push(Game { id, name: d.name, launcher: d.launcher, exe_path: d.exe_path, install_dir: d.install_dir, last_played: d.last_played, settings, running: false, running_pids: vec![], memory_bytes: 0 });
        }
        for c in &store.custom {
            let settings = store.settings.get(&c.id).cloned().unwrap_or_default();
            out.push(Game { id: c.id.clone(), name: c.name.clone(), launcher: Launcher::Custom, exe_path: Some(c.exe_path.clone()), install_dir: None, last_played: None, settings, running: false, running_pids: vec![], memory_bytes: 0 });
        }
        out.retain(|g| !g.settings.hidden);
        out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
        out
    }

    pub fn add_custom(&self, name: &str, exe_path: &str) -> Result<CustomGame> {
        let p = Path::new(exe_path);
        let name = name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(Error::Invalid("Enter a game name (up to 80 characters).".into()));
        }
        if !p.is_absolute() || !exe_path.to_ascii_lowercase().ends_with(".exe") || exe_path.chars().any(|c| c.is_control()) {
            return Err(Error::Invalid("Choose the game's .exe file.".into()));
        }
        if !p.is_file() {
            return Err(Error::NotFound("That file does not exist.".into()));
        }
        let mut s = self.store();
        if s.custom.iter().any(|c| same_path(&c.exe_path, exe_path)) {
            return Err(Error::Invalid("That executable is already in your library.".into()));
        }
        let g = CustomGame { id: game_id(Launcher::Custom, exe_path), name: name.into(), exe_path: exe_path.into() };
        s.custom.push(g.clone());
        self.save(&s)?;
        Ok(g)
    }

    pub fn remove_custom(&self, id: &str) -> Result<()> {
        let mut s = self.store();
        let before = s.custom.len();
        s.custom.retain(|c| c.id != id);
        if s.custom.len() == before {
            return Err(Error::NotFound("Only manually added games can be removed. Hide launcher games instead.".into()));
        }
        s.settings.remove(id);
        self.save(&s)
    }

    pub fn update_settings(&self, id: &str, f: impl FnOnce(&mut GameSettings)) -> Result<GameSettings> {
        let mut s = self.store();
        let e = s.settings.entry(id.to_string()).or_default();
        f(e);
        let out = e.clone();
        self.save(&s)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(exe: Option<&str>, dir: Option<&str>) -> Game {
        Game { id: "x".into(), name: "G".into(), launcher: Launcher::Steam, exe_path: exe.map(Into::into), install_dir: dir.map(Into::into), last_played: None, settings: Default::default(), running: false, running_pids: vec![], memory_bytes: 0 }
    }

    #[test]
    fn ids_are_stable_and_case_insensitive() {
        assert_eq!(game_id(Launcher::Steam, "C:\\A\\b"), game_id(Launcher::Steam, "c:/a/B"));
        assert_ne!(game_id(Launcher::Steam, "a"), game_id(Launcher::Epic, "a"));
    }

    #[test]
    fn process_matching() {
        let g = game(Some("C:\\Games\\X\\game.exe"), Some("C:\\Games\\X"));
        assert!(g.exe_path.is_some());
        assert!(process_matches(&g, Some("c:\\games\\x\\game.exe"), "game.exe"));
        assert!(process_matches(&g, Some("C:\\Games\\X\\bin\\x64\\game_dx12.exe"), "game_dx12.exe"));
        assert!(!process_matches(&g, Some("C:\\Games\\X\\CrashHandler.exe"), "CrashHandler.exe"));
        assert!(!process_matches(&g, Some("C:\\Games\\XOther\\game.exe"), "game.exe"));
        assert!(process_matches(&g, None, "GAME.exe"));
        assert!(!process_matches(&g, None, "other.exe"));
    }

    #[test]
    fn library_custom_games_and_settings_persist() {
        let d = tempfile::tempdir().unwrap();
        let lib = Library::new(DataDir::new(d.path()).unwrap());
        assert!(lib.add_custom("", "C:\\a.exe").is_err());
        assert!(lib.add_custom("A", "relative.exe").is_err());
        let exe = d.path().join("Game.exe");
        std::fs::write(&exe, b"x").unwrap();
        let exe_s = exe.to_string_lossy().into_owned();
        if Path::new(&exe_s).is_absolute() {
            let g = lib.add_custom("My Game", &exe_s).unwrap();
            assert!(lib.add_custom("Dup", &exe_s).is_err());
            lib.update_settings(&g.id, |s| {
                s.profile_id = Some("competitive".into());
                s.auto_boost = true;
            })
            .unwrap();
            let games = lib.build(vec![]);
            assert_eq!(games.len(), 1);
            assert_eq!(games[0].settings.profile_id.as_deref(), Some("competitive"));
            lib.remove_custom(&g.id).unwrap();
            assert!(lib.build(vec![]).is_empty());
        }
    }

    #[test]
    fn settings_survive_redetection() {
        let d = tempfile::tempdir().unwrap();
        let lib = Library::new(DataDir::new(d.path()).unwrap());
        let det = || vec![Detected { name: "CS2".into(), launcher: Launcher::Steam, exe_path: None, install_dir: Some("D:\\Steam\\steamapps\\common\\CS2".into()), last_played: None }];
        let id = lib.build(det())[0].id.clone();
        lib.update_settings(&id, |s| s.auto_boost = true).unwrap();
        assert!(lib.build(det())[0].settings.auto_boost);
    }
}
