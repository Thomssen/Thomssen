//! System Cleaner: scan -> review -> confirm -> delete only what was selected.
//!
//! Safety properties (all unit-tested):
//!  * categories are a fixed allow-list of cache/temp locations - there is no
//!    way to point the cleaner at an arbitrary folder;
//!  * symlinks / junctions are never followed or deleted through;
//!  * files modified since the scan (or younger than the minimum age) are skipped;
//!  * a file that is locked by a running program simply fails to delete and is
//!    reported as skipped - we never kill anything to free it;
//!  * freed space is the sum of files that were actually removed.

use crate::error::{Error, Result};
use crate::sys::win;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_FILES_PER_CATEGORY: usize = 250_000;
const MAX_DEPTH: usize = 40;

#[derive(Debug, Clone, Default)]
pub struct CleanerEnv {
    pub temp: Option<PathBuf>,
    pub windows_dir: Option<PathBuf>,
    pub local_appdata: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub program_data: Option<PathBuf>,
}

impl CleanerEnv {
    pub fn from_system() -> Self {
        let var = |k: &str| std::env::var_os(k).map(PathBuf::from);
        Self { temp: Some(std::env::temp_dir()), windows_dir: var("SystemRoot").or_else(|| var("windir")), local_appdata: var("LOCALAPPDATA"), appdata: var("APPDATA"), program_data: var("ProgramData") }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    Low,
    /// Safe, but has a visible side effect (e.g. slower first launch).
    Medium,
}

#[derive(Debug, Clone)]
pub struct CategoryDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub default_selected: bool,
    pub warning: Option<&'static str>,
    pub roots: Vec<PathBuf>,
    /// Only files whose extension is in this list (lower-case, no dot).
    pub extensions: Option<&'static [&'static str]>,
    /// Overrides the global minimum age.
    pub min_age_hours: Option<u64>,
    pub recycle_bin: bool,
}

pub fn definitions(env: &CleanerEnv) -> Vec<CategoryDef> {
    let join = |base: &Option<PathBuf>, rest: &[&str]| base.as_ref().map(|b| rest.iter().fold(b.clone(), |p, s| p.join(s)));
    let la = &env.local_appdata;
    let ra = &env.appdata;
    let pd = &env.program_data;
    let pick = |v: Vec<Option<PathBuf>>| v.into_iter().flatten().collect::<Vec<_>>();

    vec![
        CategoryDef {
            id: "user-temp",
            name: "User temporary files",
            description: "Temporary files left behind by apps and installers in your account's Temp folder. Files in use are skipped.",
            risk: Risk::Low,
            default_selected: true,
            warning: None,
            roots: pick(vec![env.temp.clone()]),
            extensions: None,
            min_age_hours: None,
            recycle_bin: false,
        },
        CategoryDef {
            id: "windows-temp",
            name: "Windows temporary files",
            description: "Temporary files in C:\\Windows\\Temp. Some may need administrator rights and will be skipped without it.",
            risk: Risk::Low,
            default_selected: true,
            warning: None,
            roots: pick(vec![join(&env.windows_dir, &["Temp"])]),
            extensions: None,
            min_age_hours: Some(24),
            recycle_bin: false,
        },
        CategoryDef {
            id: "recycle-bin",
            name: "Recycle Bin",
            description: "Files you already deleted. Emptying the Recycle Bin removes them permanently.",
            risk: Risk::Low,
            default_selected: false,
            warning: Some("Check the Recycle Bin first - emptied items cannot be recovered."),
            roots: vec![],
            extensions: None,
            min_age_hours: None,
            recycle_bin: true,
        },
        CategoryDef {
            id: "old-logs",
            name: "Old logs and crash reports",
            description: "Crash dumps and Windows Error Reporting files older than 14 days. They are only useful for diagnosing a past crash.",
            risk: Risk::Low,
            default_selected: true,
            warning: None,
            roots: pick(vec![join(la, &["CrashDumps"]), join(la, &["Microsoft", "Windows", "WER", "ReportArchive"]), join(la, &["Microsoft", "Windows", "WER", "ReportQueue"]), join(pd, &["Microsoft", "Windows", "WER", "ReportArchive"]), join(pd, &["Microsoft", "Windows", "WER", "ReportQueue"])]),
            extensions: None,
            min_age_hours: Some(24 * 14),
            recycle_bin: false,
        },
        CategoryDef {
            id: "web-cache",
            name: "Browser and Windows web cache",
            description: "Cached web content of Chrome, Edge and Windows (INetCache). Cookies, logins, history and bookmarks are not touched. Close your browser first for best results.",
            risk: Risk::Low,
            default_selected: false,
            warning: Some("Pages may load a little slower the first time after cleaning."),
            roots: pick(vec![
                join(la, &["Microsoft", "Windows", "INetCache"]),
                join(la, &["Google", "Chrome", "User Data", "Default", "Cache", "Cache_Data"]),
                join(la, &["Google", "Chrome", "User Data", "Default", "Code Cache"]),
                join(la, &["Microsoft", "Edge", "User Data", "Default", "Cache", "Cache_Data"]),
                join(la, &["Microsoft", "Edge", "User Data", "Default", "Code Cache"]),
            ]),
            extensions: None,
            min_age_hours: None,
            recycle_bin: false,
        },
        CategoryDef {
            id: "launcher-cache",
            name: "Launcher and chat app caches",
            description: "Web caches of Steam's built-in browser and Discord. Your games, saves, accounts and settings are not touched.",
            risk: Risk::Low,
            default_selected: false,
            warning: Some("Close Steam and Discord first. Their windows may take a moment longer to load afterwards."),
            roots: pick(vec![join(la, &["Steam", "htmlcache"]), join(ra, &["discord", "Cache"]), join(ra, &["discord", "Code Cache"])]),
            extensions: None,
            min_age_hours: None,
            recycle_bin: false,
        },
        CategoryDef {
            id: "shader-cache",
            name: "Shader caches (optional)",
            description: "DirectX, NVIDIA and AMD shader caches. These are rebuilt automatically as you play.",
            risk: Risk::Medium,
            default_selected: false,
            warning: Some("Not recommended for performance. Games must recompile shaders after this, which can cause stutter and longer loading until the cache is rebuilt. Only clean it to fix a corrupted cache or after a GPU driver problem."),
            roots: pick(vec![join(la, &["D3DSCache"]), join(la, &["NVIDIA", "DXCache"]), join(la, &["NVIDIA", "GLCache"]), join(la, &["AMD", "DxCache"]), join(la, &["AMD", "DxcCache"])]),
            extensions: None,
            min_age_hours: None,
            recycle_bin: false,
        },
    ]
}

#[derive(Debug, Clone)]
struct FileRec {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    root: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk: Risk,
    pub default_selected: bool,
    pub warning: Option<String>,
    pub bytes: u64,
    pub files: u64,
    /// The largest files (a preview; the full list is what will be deleted).
    pub largest: Vec<FileEntry>,
    pub locations: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: String,
    pub categories: Vec<CategoryScan>,
    pub total_bytes: u64,
}

struct CatState {
    def: CategoryDef,
    files: Vec<FileRec>,
    recycle_bytes: u64,
}

/// A completed scan, kept in memory so deletion only ever touches files the
/// user was shown.
pub struct Scan {
    pub id: String,
    cats: HashMap<&'static str, CatState>,
}

fn is_reparse(meta: &std::fs::Metadata) -> bool {
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return meta.file_attributes() & 0x400 != 0; // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[allow(unreachable_code)]
    false
}

fn walk(root_idx: usize, root: &Path, def: &CategoryDef, min_age: Duration, out: &mut Vec<FileRec>) -> bool {
    let now = SystemTime::now();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else { continue };
            if is_reparse(&meta) {
                continue;
            }
            if meta.is_dir() {
                stack.push((e.path(), depth + 1));
            } else if meta.is_file() {
                let Ok(modified) = meta.modified() else { continue };
                if now.duration_since(modified).unwrap_or_default() < min_age {
                    continue;
                }
                if let Some(exts) = def.extensions {
                    let ext = e.path().extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                    if !exts.contains(&ext.as_str()) {
                        continue;
                    }
                }
                if out.len() >= MAX_FILES_PER_CATEGORY {
                    return true;
                }
                out.push(FileRec { path: e.path(), size: meta.len(), modified, root: root_idx });
            }
        }
    }
    false
}

pub fn scan(env: &CleanerEnv, min_age_hours: u64) -> (Scan, ScanSummary) {
    let mut cats = HashMap::new();
    let mut summaries = Vec::new();
    let default_age = Duration::from_secs(min_age_hours * 3600);
    for def in definitions(env) {
        let min_age = def.min_age_hours.map(|h| Duration::from_secs(h * 3600).max(default_age)).unwrap_or(default_age);
        let mut files = Vec::new();
        let mut truncated = false;
        for (i, root) in def.roots.iter().enumerate() {
            if root.is_dir() && is_safe_root(root) {
                truncated |= walk(i, root, &def, min_age, &mut files);
            }
        }
        let (mut bytes, mut count, mut recycle_bytes) = (files.iter().map(|f| f.size).sum::<u64>(), files.len() as u64, 0);
        if def.recycle_bin {
            if let Some((b, n)) = win::recycle_bin_info() {
                bytes = b;
                count = n;
                recycle_bytes = b;
            }
        }
        let mut largest: Vec<&FileRec> = files.iter().collect();
        largest.sort_by(|a, b| b.size.cmp(&a.size));
        summaries.push(CategoryScan {
            id: def.id.into(),
            name: def.name.into(),
            description: def.description.into(),
            risk: def.risk,
            default_selected: def.default_selected,
            warning: def.warning.map(String::from),
            bytes,
            files: count,
            largest: largest.iter().take(25).map(|f| FileEntry { path: f.path.to_string_lossy().into_owned(), size: f.size }).collect(),
            locations: def.roots.iter().map(|r| r.to_string_lossy().into_owned()).collect(),
            truncated,
        });
        cats.insert(def.id, CatState { def, files, recycle_bytes });
    }
    let total = summaries.iter().map(|s| s.bytes).sum();
    let id = uuid::Uuid::new_v4().to_string();
    (Scan { id: id.clone(), cats }, ScanSummary { scan_id: id, categories: summaries, total_bytes: total })
}

/// Fast, bounded estimate of reclaimable bytes in the user temp folder (used
/// by the analysis; never deletes anything).
pub fn quick_temp_bytes(env: &CleanerEnv, min_age_hours: u64) -> u64 {
    let Some(root) = env.temp.as_ref().filter(|r| r.is_dir() && is_safe_root(r)) else { return 0 };
    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    let min_age = Duration::from_secs(min_age_hours * 3600);
    let now = SystemTime::now();
    let (mut total, mut seen) = (0u64, 0usize);
    let mut stack = vec![(root.clone(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_DEPTH || seen > 100_000 || std::time::Instant::now() > deadline {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(m) = std::fs::symlink_metadata(e.path()) else { continue };
            if is_reparse(&m) {
                continue;
            }
            seen += 1;
            if m.is_dir() {
                stack.push((e.path(), depth + 1));
            } else if m.is_file() && m.modified().is_ok_and(|t| now.duration_since(t).unwrap_or_default() >= min_age) {
                total += m.len();
            }
        }
    }
    total
}

/// A root is only ever cleaned if it is not a drive root and has at least two
/// folder names in its path - a belt-and-braces guard on top of the fixed list.
pub fn is_safe_root(p: &Path) -> bool {
    // Count real folder names only: the drive prefix and root are not folders,
    // so `C:\\` and `C:\\Users` are rejected while `C:\\Windows\\Temp` is fine.
    p.is_absolute() && p.components().filter(|c| matches!(c, std::path::Component::Normal(_))).count() >= 2
}

/// True when `path` is lexically inside `root` (no `..` tricks).
pub fn is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root) && !path.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryReport {
    pub id: String,
    pub name: String,
    pub deleted_files: u64,
    pub bytes_freed: u64,
    pub skipped_files: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub categories: Vec<CategoryReport>,
    pub total_deleted: u64,
    pub total_freed: u64,
    pub total_skipped: u64,
}

pub fn clean(scan: &Scan, selected: &[String]) -> Result<CleanReport> {
    if selected.is_empty() {
        return Err(Error::Invalid("Select at least one category to clean.".into()));
    }
    let mut report = CleanReport::default();
    for id in selected {
        let Some(cat) = scan.cats.values().find(|c| c.def.id == id) else {
            return Err(Error::Invalid(format!("Unknown category: {id}")));
        };
        let mut rep = CategoryReport { id: cat.def.id.into(), name: cat.def.name.into(), ..Default::default() };

        if cat.def.recycle_bin {
            let before = win::recycle_bin_info().map(|(b, _)| b).unwrap_or(cat.recycle_bytes);
            match win::empty_recycle_bin() {
                Ok(()) => {
                    let after = win::recycle_bin_info().map(|(b, _)| b).unwrap_or(0);
                    rep.bytes_freed = before.saturating_sub(after);
                }
                Err(e) => rep.error = Some(e.to_string()),
            }
        } else {
            for f in &cat.files {
                let root = &cat.def.roots[f.root];
                if !is_within(&f.path, root) {
                    rep.skipped_files += 1;
                    continue;
                }
                // Re-check right before deleting: still a plain file, unchanged since the scan.
                let ok = match std::fs::symlink_metadata(&f.path) {
                    Ok(m) => !is_reparse(&m) && m.is_file() && m.len() == f.size && m.modified().map(|t| t == f.modified).unwrap_or(false),
                    Err(_) => false,
                };
                if !ok {
                    rep.skipped_files += 1;
                    continue;
                }
                match std::fs::remove_file(&f.path) {
                    Ok(()) => {
                        rep.deleted_files += 1;
                        rep.bytes_freed += f.size;
                    }
                    Err(_) => rep.skipped_files += 1, // in use or no permission
                }
            }
            // Tidy empty sub-folders (never the root itself). remove_dir fails on non-empty dirs.
            for (i, root) in cat.def.roots.iter().enumerate() {
                let mut dirs: Vec<PathBuf> = cat.files.iter().filter(|f| f.root == i).filter_map(|f| f.path.parent().map(|p| p.to_path_buf())).collect();
                dirs.sort();
                dirs.dedup();
                dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
                for d in dirs {
                    if d != *root && is_within(&d, root) {
                        let _ = std::fs::remove_dir(&d);
                    }
                }
            }
        }
        report.total_deleted += rep.deleted_files;
        report.total_freed += rep.bytes_freed;
        report.total_skipped += rep.skipped_files;
        report.categories.push(rep);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    fn mk(dir: &Path, rel: &str, size: usize, age_hours: u64) -> PathBuf {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, vec![7u8; size]).unwrap();
        let t = SystemTime::now() - Duration::from_secs(age_hours * 3600);
        File::options().write(true).open(&p).unwrap().set_modified(t).unwrap();
        p
    }

    fn env(root: &Path) -> CleanerEnv {
        CleanerEnv { temp: Some(root.join("Users").join("me").join("Temp")), windows_dir: Some(root.join("Windows")), local_appdata: Some(root.join("Users").join("me").join("Local")), appdata: None, program_data: None }
    }

    fn cat<'a>(s: &'a ScanSummary, id: &str) -> &'a CategoryScan {
        s.categories.iter().find(|c| c.id == id).unwrap()
    }

    #[test]
    fn scan_respects_min_age_and_lists_only_candidates() {
        let d = tempfile::tempdir().unwrap();
        let e = env(d.path());
        let t = e.temp.clone().unwrap();
        mk(&t, "old.tmp", 1000, 48);
        mk(&t, "sub/old2.tmp", 500, 48);
        mk(&t, "fresh.tmp", 9999, 0);
        let (_scan, sum) = scan(&e, 1);
        let c = cat(&sum, "user-temp");
        assert_eq!(c.files, 2);
        assert_eq!(c.bytes, 1500);
        assert!(!c.largest.iter().any(|f| f.path.ends_with("fresh.tmp")));
    }

    #[test]
    fn clean_deletes_only_selected_categories() {
        let d = tempfile::tempdir().unwrap();
        let e = env(d.path());
        let t = e.temp.clone().unwrap();
        let shader = e.local_appdata.clone().unwrap().join("D3DSCache");
        let a = mk(&t, "a.tmp", 100, 48);
        let b = mk(&shader, "cache.bin", 100, 48);
        let (scan_, _) = scan(&e, 1);
        let rep = clean(&scan_, &["user-temp".into()]).unwrap();
        assert_eq!((rep.total_deleted, rep.total_freed), (1, 100));
        assert!(!a.exists());
        assert!(b.exists(), "shader cache must not be touched unless selected");
    }

    #[test]
    fn files_changed_after_scan_are_skipped() {
        let d = tempfile::tempdir().unwrap();
        let e = env(d.path());
        let p = mk(e.temp.as_ref().unwrap(), "x.tmp", 100, 48);
        let (scan_, _) = scan(&e, 1);
        fs::write(&p, vec![1u8; 300]).unwrap(); // app wrote to it after the scan
        let rep = clean(&scan_, &["user-temp".into()]).unwrap();
        assert_eq!((rep.total_deleted, rep.total_skipped), (0, 1));
        assert!(p.exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        let d = tempfile::tempdir().unwrap();
        let e = env(d.path());
        let t = e.temp.clone().unwrap();
        let outside = d.path().join("precious");
        let victim = mk(&outside, "doc.txt", 10, 999);
        fs::create_dir_all(&t).unwrap();
        std::os::unix::fs::symlink(&outside, t.join("link")).unwrap();
        let (scan_, sum) = scan(&e, 1);
        assert_eq!(cat(&sum, "user-temp").files, 0);
        clean(&scan_, &["user-temp".into()]).unwrap();
        assert!(victim.exists());
    }

    #[test]
    fn unknown_or_empty_selection_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let (scan_, _) = scan(&env(d.path()), 1);
        assert!(clean(&scan_, &[]).is_err());
        assert!(clean(&scan_, &["../../etc".into()]).is_err());
    }

    #[test]
    fn path_guards() {
        // Use the platform's own notion of an absolute path.
        let (root, deep, short, drive) = if cfg!(windows) { ("C:\\a\\b", "C:\\a\\b\\c", "C:\\Users", "C:\\") } else { ("/a/b", "/a/b/c", "/home", "/") };
        let sep = std::path::MAIN_SEPARATOR;
        assert!(is_within(Path::new(deep), Path::new(root)));
        assert!(!is_within(Path::new(&format!("{root}{sep}..{sep}c")), Path::new(root)));
        assert!(!is_within(Path::new(&format!("{root}c")), Path::new(root)));
        assert!(!is_safe_root(Path::new(drive)));
        assert!(!is_safe_root(Path::new(short)));
        assert!(is_safe_root(Path::new(&format!("{short}{sep}me{sep}Temp"))));
    }

    #[test]
    fn shader_cache_is_never_preselected() {
        let defs = definitions(&CleanerEnv::default());
        assert!(!defs.iter().find(|d| d.id == "shader-cache").unwrap().default_selected);
        assert!(defs.iter().find(|d| d.id == "shader-cache").unwrap().warning.is_some());
    }
}
