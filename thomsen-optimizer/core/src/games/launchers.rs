//! Read-only detection of installed games from launcher manifests.
//! Steam and Epic are fully manifest-driven. The Xbox app and Battle.net do
//! not publish a documented manifest, so those two use best-effort folder /
//! uninstall-registry scans and may miss titles - the "Add game" button covers
//! the rest.

use super::vdf;
use super::{is_auxiliary_exe, Detected, Launcher};
use crate::sys::{Hive, SystemOps};
use chrono::{TimeZone, Utc};
use std::path::{Path, PathBuf};

/// Well-known games whose main executable the "largest .exe" heuristic would
/// otherwise get wrong. Keyed by Steam app id.
const KNOWN_EXE: &[(&str, &str)] = &[("730", "game\\bin\\win64\\cs2.exe"), ("271590", "GTA5.exe"), ("1172470", "r5apex.exe"), ("570", "game\\bin\\win64\\dota2.exe"), ("359550", "RainbowSix.exe")];

/// Steam tools that show up as "installed apps" but are not games.
fn steam_non_game(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("redistributable") || n.starts_with("steamworks") || n.starts_with("steam linux runtime") || n.starts_with("proton") || n.contains("soundtrack") || n.contains("dedicated server") || n == "steam controller configs"
}

/// Best guess of a game's main executable: the largest non-helper .exe within
/// three folder levels. Bounded so a huge install can never stall the UI.
pub fn find_main_exe(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(u64, PathBuf)> = None;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    let mut visited = 0;
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            visited += 1;
            if visited > 4000 {
                return best.map(|b| b.1);
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            let p = e.path();
            if ft.is_dir() {
                if depth < 3 {
                    stack.push((p, depth + 1));
                }
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                if is_auxiliary_exe(&name) {
                    continue;
                }
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                if best.as_ref().map_or(true, |(s, _)| size > *s) {
                    best = Some((size, p));
                }
            }
        }
    }
    best.map(|b| b.1)
}

pub fn steam_from_dir(steam_dir: &Path) -> Vec<Detected> {
    let mut libs: Vec<PathBuf> = vec![steam_dir.to_path_buf()];
    if let Ok(t) = std::fs::read_to_string(steam_dir.join("steamapps").join("libraryfolders.vdf")) {
        if let Some(v) = vdf::parse(&t) {
            if let Some(lf) = v.get("libraryfolders") {
                for (_, e) in lf.entries() {
                    if let Some(p) = e.str_at("path") {
                        let pb = PathBuf::from(p);
                        if !libs.iter().any(|l| l == &pb) {
                            libs.push(pb);
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for lib in libs {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let fname = e.file_name().to_string_lossy().into_owned();
            if !(fname.starts_with("appmanifest_") && fname.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
            let Some(v) = vdf::parse(&text) else { continue };
            let Some(st) = v.get("AppState") else { continue };
            let (Some(appid), Some(name), Some(installdir)) = (st.str_at("appid"), st.str_at("name"), st.str_at("installdir")) else { continue };
            let flags: u32 = st.str_at("StateFlags").and_then(|s| s.parse().ok()).unwrap_or(0);
            if flags & 4 == 0 || steam_non_game(name) {
                continue;
            }
            let dir = apps.join("common").join(installdir);
            if !dir.is_dir() {
                continue;
            }
            let known = KNOWN_EXE.iter().find(|(id, _)| *id == appid).map(|(_, rel)| dir.join(rel)).filter(|p| p.is_file());
            let exe = known.or_else(|| find_main_exe(&dir));
            let last = st.str_at("LastPlayed").and_then(|s| s.parse::<i64>().ok()).filter(|t| *t > 0).and_then(|t| Utc.timestamp_opt(t, 0).single());
            out.push(Detected { name: name.to_string(), launcher: Launcher::Steam, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(dir.to_string_lossy().into_owned()), last_played: last });
        }
    }
    out
}

pub fn steam(ops: &dyn SystemOps) -> Vec<Detected> {
    let path = ops
        .reg_get(Hive::CurrentUser, "Software\\Valve\\Steam", "SteamPath")
        .ok()
        .flatten()
        .and_then(|d| d.as_str().map(String::from))
        .or_else(|| ops.reg_get(Hive::LocalMachine, "SOFTWARE\\WOW6432Node\\Valve\\Steam", "InstallPath").ok().flatten().and_then(|d| d.as_str().map(String::from)));
    match path {
        Some(p) => steam_from_dir(Path::new(&p.replace('/', "\\"))),
        None => vec![],
    }
}

pub fn epic_from_manifests(dir: &Path) -> Vec<Detected> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut out = Vec::new();
    for e in rd.flatten() {
        if !e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("item")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(e.path()) else { continue };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty());
        let (Some(name), Some(loc)) = (s("DisplayName"), s("InstallLocation")) else { continue };
        if v.get("bIsIncompleteInstall").and_then(|x| x.as_bool()).unwrap_or(false) {
            continue;
        }
        let is_game = v.get("AppCategories").and_then(|c| c.as_array()).map_or(true, |a| a.iter().any(|c| c.as_str() == Some("games")));
        if !is_game || !Path::new(loc).is_dir() {
            continue;
        }
        let exe = s("LaunchExecutable").map(|rel| Path::new(loc).join(rel.replace('\\', "/"))).filter(|p| p.is_file()).or_else(|| find_main_exe(Path::new(loc)));
        out.push(Detected { name: name.to_string(), launcher: Launcher::Epic, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(loc.to_string()), last_played: None });
    }
    out
}

pub fn epic() -> Vec<Detected> {
    match std::env::var_os("ProgramData") {
        Some(pd) => epic_from_manifests(&PathBuf::from(pd).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests")),
        None => vec![],
    }
}

/// Xbox app games installed to `<drive>:\XboxGames\<Title>\Content`.
pub fn xbox_from_roots(roots: &[PathBuf]) -> Vec<Detected> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.eq_ignore_ascii_case("GameSave") || name.starts_with('.') {
                continue;
            }
            let content = e.path().join("Content");
            let base = if content.is_dir() { content } else { e.path() };
            let exe = find_main_exe(&base);
            out.push(Detected { name, launcher: Launcher::Xbox, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(e.path().to_string_lossy().into_owned()), last_played: None });
        }
    }
    out
}

pub fn xbox() -> Vec<Detected> {
    let roots: Vec<PathBuf> = (b'C'..=b'Z').map(|l| PathBuf::from(format!("{}:\\XboxGames", l as char))).filter(|p| p.is_dir()).collect();
    xbox_from_roots(&roots)
}

pub fn battlenet(ops: &dyn SystemOps) -> Vec<Detected> {
    let base = "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let mut out = Vec::new();
    for k in ops.reg_subkeys(Hive::LocalMachine, base).unwrap_or_default() {
        let path = format!("{base}\\{k}");
        let get = |n: &str| ops.reg_get(Hive::LocalMachine, &path, n).ok().flatten().and_then(|d| d.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty());
        let (Some(publisher), Some(name), Some(loc)) = (get("Publisher"), get("DisplayName"), get("InstallLocation")) else { continue };
        let p = publisher.to_ascii_lowercase();
        if !(p.contains("blizzard") || p.contains("activision")) || name.eq_ignore_ascii_case("Battle.net") || !Path::new(&loc).is_dir() {
            continue;
        }
        let exe = find_main_exe(Path::new(&loc));
        out.push(Detected { name, launcher: Launcher::BattleNet, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(loc), last_played: None });
    }
    out
}

pub fn detect_all(ops: &dyn SystemOps) -> Vec<Detected> {
    let mut v = steam(ops);
    v.extend(epic());
    v.extend(xbox());
    v.extend(battlenet(ops));
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(p: &Path, size: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![0u8; size]).unwrap();
    }

    #[test]
    fn steam_manifests_across_libraries() {
        let d = tempfile::tempdir().unwrap();
        let steam = d.path().join("Steam");
        let lib2 = d.path().join("Lib2");
        fs::create_dir_all(steam.join("steamapps")).unwrap();
        fs::write(
            steam.join("steamapps/libraryfolders.vdf"),
            format!("\"libraryfolders\"\n{{\n \"0\" {{ \"path\" \"{}\" }}\n \"1\" {{ \"path\" \"{}\" }}\n}}", steam.to_string_lossy().replace('\\', "\\\\"), lib2.to_string_lossy().replace('\\', "\\\\")),
        )
        .unwrap();
        // Fully installed game in the main library, with helper exes around it.
        fs::write(steam.join("steamapps/appmanifest_100.acf"), "\"AppState\" { \"appid\" \"100\" \"name\" \"Test Game\" \"installdir\" \"TestGame\" \"StateFlags\" \"4\" \"LastPlayed\" \"1700000000\" }").unwrap();
        touch(&steam.join("steamapps/common/TestGame/bin/game.exe"), 5000);
        touch(&steam.join("steamapps/common/TestGame/UnityCrashHandler64.exe"), 9000);
        touch(&steam.join("steamapps/common/TestGame/unins000.exe"), 9999);
        // Still downloading (StateFlags 1026) -> ignored.
        fs::write(steam.join("steamapps/appmanifest_200.acf"), "\"AppState\" { \"appid\" \"200\" \"name\" \"Half\" \"installdir\" \"Half\" \"StateFlags\" \"1026\" }").unwrap();
        touch(&steam.join("steamapps/common/Half/h.exe"), 10);
        // Tool -> ignored. Game in second library.
        fs::write(steam.join("steamapps/appmanifest_228980.acf"), "\"AppState\" { \"appid\" \"228980\" \"name\" \"Steamworks Common Redistributables\" \"installdir\" \"Redist\" \"StateFlags\" \"4\" }").unwrap();
        touch(&steam.join("steamapps/common/Redist/r.exe"), 10);
        fs::create_dir_all(lib2.join("steamapps")).unwrap();
        fs::write(lib2.join("steamapps/appmanifest_300.acf"), "\"AppState\" { \"appid\" \"300\" \"name\" \"Second\" \"installdir\" \"Second\" \"StateFlags\" \"4\" }").unwrap();
        touch(&lib2.join("steamapps/common/Second/second.exe"), 100);

        let mut g = steam_from_dir(&steam);
        g.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(g.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Second", "Test Game"]);
        let tg = &g[1];
        assert!(tg.exe_path.as_deref().unwrap().ends_with("game.exe"), "{:?}", tg.exe_path);
        assert_eq!(tg.last_played.unwrap().timestamp(), 1_700_000_000);
        assert!(g[0].last_played.is_none());
    }

    #[test]
    fn epic_manifests() {
        let d = tempfile::tempdir().unwrap();
        let game = d.path().join("Games/Fortnite");
        touch(&game.join("FortniteGame/Binaries/Win64/FortniteClient-Win64-Shipping.exe"), 100);
        let man = d.path().join("Manifests");
        fs::create_dir_all(&man).unwrap();
        let item = |name: &str, loc: &Path, exe: &str, cats: &str, incomplete: bool| {
            format!("{{\"DisplayName\":\"{name}\",\"InstallLocation\":{},\"LaunchExecutable\":\"{exe}\",\"bIsIncompleteInstall\":{incomplete},\"AppCategories\":{cats}}}", serde_json::to_string(&loc.to_string_lossy()).unwrap())
        };
        fs::write(man.join("a.item"), item("Fortnite", &game, "FortniteGame/Binaries/Win64/FortniteClient-Win64-Shipping.exe", "[\"public\",\"games\"]", false)).unwrap();
        fs::write(man.join("b.item"), item("Some Tool", &game, "x.exe", "[\"applications\"]", false)).unwrap();
        fs::write(man.join("c.item"), item("Broken", &game, "x.exe", "[\"games\"]", true)).unwrap();
        fs::write(man.join("d.item"), "not json").unwrap();
        let g = epic_from_manifests(&man);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].name, "Fortnite");
        assert!(g[0].exe_path.as_deref().unwrap().ends_with("FortniteClient-Win64-Shipping.exe"));
    }

    #[test]
    fn xbox_scan() {
        let d = tempfile::tempdir().unwrap();
        touch(&d.path().join("Minecraft/Content/Minecraft.Windows.exe"), 500);
        let g = xbox_from_roots(&[d.path().to_path_buf(), d.path().join("missing")]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].launcher, Launcher::Xbox);
    }

    #[test]
    fn missing_installs_are_not_an_error() {
        assert!(steam_from_dir(Path::new("/definitely/not/here")).is_empty());
        assert!(epic_from_manifests(Path::new("/definitely/not/here")).is_empty());
    }
}
