//! FPS measurement through Intel's PresentMon (an ETW-based frame-presentation
//! monitor). Thomsen Optimizer does not inject into games, hook graphics APIs
//! or read game memory - it only reads PresentMon's CSV output - so anti-cheat
//! systems are not involved. Nothing here is simulated: with no PresentMon,
//! or when the game's presents are not visible, FPS is simply "unavailable".

use crate::error::{Error, Result};
use crate::profiles::valid_exe_name;
use crate::sys::proc_util::command;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_FRAMES: usize = 3_000_000;
const MIN_FRAMES_FOR_LOW: usize = 100;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameStats {
    pub frames: usize,
    pub duration_secs: f64,
    pub avg_fps: f64,
    /// `None` until at least 100 frames were captured.
    pub one_percent_low_fps: Option<f64>,
    pub avg_frametime_ms: f64,
    pub p99_frametime_ms: f64,
    pub max_frametime_ms: f64,
    pub stddev_ms: f64,
    /// 100 = perfectly even frame pacing; `1 - stddev/mean`, clamped to 0-100.
    pub stability_pct: f64,
}

/// Statistics over frame times in milliseconds.
///  * average FPS = frames / total time;
///  * 1% low FPS = 1000 / (mean of the slowest 1% of frame times);
///  * stability = 100 * (1 - stddev / mean).
pub fn stats(frame_times_ms: &[f32]) -> Option<FrameStats> {
    let v: Vec<f64> = frame_times_ms.iter().map(|x| *x as f64).filter(|x| x.is_finite() && *x > 0.0).collect();
    if v.is_empty() {
        return None;
    }
    let n = v.len();
    let total: f64 = v.iter().sum();
    let mean = total / n as f64;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let sd = var.sqrt();
    let mut sorted = v.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let worst_n = ((n as f64 * 0.01).ceil() as usize).max(1);
    let worst_mean = sorted[..worst_n].iter().sum::<f64>() / worst_n as f64;
    let p99_idx = (((n as f64) * 0.01).floor() as usize).min(n - 1);
    Some(FrameStats {
        frames: n,
        duration_secs: total / 1000.0,
        avg_fps: 1000.0 / mean,
        one_percent_low_fps: (n >= MIN_FRAMES_FOR_LOW).then(|| 1000.0 / worst_mean),
        avg_frametime_ms: mean,
        p99_frametime_ms: sorted[p99_idx],
        max_frametime_ms: sorted[0],
        stddev_ms: sd,
        stability_pct: (100.0 * (1.0 - sd / mean)).clamp(0.0, 100.0),
    })
}

/// Averages `v` into at most `max` buckets, for graphing.
pub fn downsample(v: &[f32], max: usize) -> Vec<f32> {
    if v.len() <= max || max == 0 {
        return v.to_vec();
    }
    let step = v.len() as f64 / max as f64;
    (0..max)
        .map(|i| {
            let a = (i as f64 * step) as usize;
            let b = (((i + 1) as f64 * step) as usize).clamp(a + 1, v.len());
            v[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect()
}

/// Incremental PresentMon CSV parser (v1 `MsBetweenPresents`, v2 `FrameTime`).
#[derive(Default)]
pub struct CsvParser {
    col: Option<usize>,
}

impl CsvParser {
    /// Returns a frame time in ms for data lines; `None` for headers/junk.
    pub fn feed(&mut self, line: &str) -> Option<f32> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if self.col.is_none() {
            let idx = fields.iter().position(|f| {
                let f = f.trim().to_ascii_lowercase();
                f == "msbetweenpresents" || f == "frametime"
            });
            if let Some(i) = idx {
                self.col = Some(i);
            }
            return None; // header (or pre-header chatter)
        }
        let v: f32 = fields.get(self.col?)?.trim().parse().ok()?;
        (v.is_finite() && v > 0.0 && v < 60_000.0).then_some(v)
    }
}

#[derive(Default)]
struct Shared {
    frames: Vec<f32>,
    header_seen: bool,
    stderr: String,
}

pub struct Capture {
    child: Child,
    shared: Arc<Mutex<Shared>>,
    pub game_name: String,
    pub process_name: String,
    pub started_at: DateTime<Utc>,
    started: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveFps {
    pub capturing: bool,
    pub game_name: Option<String>,
    pub frames: usize,
    pub elapsed_secs: f64,
    /// Recent frame times (ms), newest last.
    pub recent: Vec<f32>,
    pub current_fps: Option<f64>,
    /// Set when PresentMon is running but has not seen any presents yet.
    pub waiting_for_frames: bool,
    pub error: Option<String>,
}

/// Where a PresentMon executable may come from: the user's chosen file or the
/// app's own tools folder - and it must be an actual PresentMon binary name.
pub fn validate_presentmon(path: &Path) -> Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !path.is_absolute() || !name.starts_with("presentmon") || !name.ends_with(".exe") {
        return Err(Error::Invalid("Choose the PresentMon console executable (for example PresentMon-2.x-x64.exe).".into()));
    }
    if !path.is_file() {
        return Err(Error::NotFound("PresentMon was not found at that location.".into()));
    }
    Ok(())
}

fn spawn(path: &Path, process: &str, v2_flags: bool) -> Result<Child> {
    let d = if v2_flags { "--" } else { "-" };
    let mut c = command(&path.to_string_lossy());
    c.args([format!("{d}process_name"), process.to_string(), format!("{d}output_stdout"), format!("{d}stop_existing_session"), format!("{d}terminate_on_proc_exit")]);
    if v2_flags {
        c.arg("--no_console_stats");
    }
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    c.spawn().map_err(|e| Error::Command(format!("Could not start PresentMon: {e}")))
}

impl Capture {
    pub fn start(presentmon: &Path, process_name: &str, game_name: &str) -> Result<Capture> {
        validate_presentmon(presentmon)?;
        if !valid_exe_name(process_name) {
            return Err(Error::Invalid("Unsupported process name.".into()));
        }
        // PresentMon 2.x uses --flags, 1.x -flags: try 2.x first and fall back
        // if it exits immediately.
        let mut child = spawn(presentmon, process_name, true)?;
        std::thread::sleep(Duration::from_millis(900));
        if matches!(child.try_wait(), Ok(Some(s)) if !s.success()) {
            child = spawn(presentmon, process_name, false)?;
        }
        let shared = Arc::new(Mutex::new(Shared::default()));
        if let Some(out) = child.stdout.take() {
            let sh = shared.clone();
            std::thread::spawn(move || {
                let mut p = CsvParser::default();
                for line in BufReader::new(out).lines().map_while(|l| l.ok()) {
                    let mut g = sh.lock().unwrap();
                    if p.col.is_some() {
                        g.header_seen = true;
                    }
                    if let Some(ft) = p.feed(&line) {
                        if g.frames.len() < MAX_FRAMES {
                            g.frames.push(ft);
                        }
                    }
                }
            });
        }
        if let Some(err) = child.stderr.take() {
            let sh = shared.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(err).lines().map_while(|l| l.ok()).take(40) {
                    let mut g = sh.lock().unwrap();
                    if g.stderr.len() < 2000 {
                        g.stderr.push_str(&line);
                        g.stderr.push('\n');
                    }
                }
            });
        }
        Ok(Capture { child, shared, game_name: game_name.into(), process_name: process_name.into(), started_at: Utc::now(), started: Instant::now() })
    }

    pub fn live(&mut self) -> LiveFps {
        let exited = matches!(self.child.try_wait(), Ok(Some(_)));
        let g = self.shared.lock().unwrap();
        let n = g.frames.len();
        let recent: Vec<f32> = g.frames[n.saturating_sub(300)..].to_vec();
        let last60 = &g.frames[n.saturating_sub(60)..];
        let current = (!last60.is_empty()).then(|| 1000.0 * last60.len() as f64 / last60.iter().map(|x| *x as f64).sum::<f64>());
        let error = if exited && n == 0 { Some(explain_failure(&g.stderr)) } else { None };
        LiveFps { capturing: !exited || n > 0, game_name: Some(self.game_name.clone()), frames: n, elapsed_secs: self.started.elapsed().as_secs_f64(), recent, current_fps: current, waiting_for_frames: n == 0 && error.is_none(), error }
    }

    /// Stops PresentMon and returns the captured frame times.
    pub fn finish(mut self) -> Vec<f32> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::thread::sleep(Duration::from_millis(100));
        let g = self.shared.lock().unwrap();
        g.frames.clone()
    }
}

pub fn explain_failure(stderr: &str) -> String {
    let l = stderr.to_ascii_lowercase();
    if l.contains("access") || l.contains("administrator") || l.contains("privilege") || l.contains("performance log users") {
        "PresentMon needs permission to read Windows frame events. Either relaunch Thomsen Optimizer as administrator for the capture, or add your user to the \"Performance Log Users\" group.".into()
    } else if stderr.trim().is_empty() {
        "PresentMon exited without capturing any frames. Make sure the game is running and renders with DirectX, Vulkan or OpenGL.".into()
    } else {
        format!("PresentMon reported: {}", stderr.lines().next().unwrap_or("").trim())
    }
}

pub fn default_tools_dir(data: &Path) -> PathBuf {
    data.join("tools")
}

/// Looks for a PresentMon console exe in the app's tools folder.
pub fn find_in_tools(data: &Path) -> Option<PathBuf> {
    std::fs::read_dir(default_tools_dir(data)).ok()?.flatten().map(|e| e.path()).find(|p| validate_presentmon(p).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_on_known_data() {
        // 100 frames at 10 ms (100 FPS) + 1 frame at 100 ms.
        let mut v = vec![10.0f32; 100];
        v.push(100.0);
        let s = stats(&v).unwrap();
        assert_eq!(s.frames, 101);
        assert!((s.avg_fps - 1000.0 * 101.0 / 1100.0).abs() < 1e-6);
        // worst 1% of 101 frames = ceil(1.01) = 2 frames: (100 + 10)/2 = 55 ms
        assert!((s.one_percent_low_fps.unwrap() - 1000.0 / 55.0).abs() < 1e-6);
        assert_eq!(s.max_frametime_ms, 100.0);
        assert!(s.stability_pct < 100.0 && s.stability_pct > 0.0);
    }

    #[test]
    fn perfectly_even_frames() {
        let s = stats(&vec![16.0f32; 500]).unwrap();
        assert!((s.avg_fps - 62.5).abs() < 1e-9);
        assert!((s.one_percent_low_fps.unwrap() - 62.5).abs() < 1e-9);
        assert_eq!(s.stability_pct, 100.0);
    }

    #[test]
    fn few_frames_do_not_report_a_one_percent_low() {
        let s = stats(&[10.0, 12.0, 11.0]).unwrap();
        assert!(s.one_percent_low_fps.is_none());
        assert!(stats(&[]).is_none());
        assert!(stats(&[0.0, -1.0, f32::NAN]).is_none());
    }

    #[test]
    fn parses_presentmon_v1_and_v2_csv() {
        let mut p = CsvParser::default();
        assert_eq!(p.feed("Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,MsBetweenPresents,MsInPresentAPI"), None);
        assert_eq!(p.feed("game.exe,1234,0x1,DXGI,0,0,0,0.5,6.94,0.1"), Some(6.94));
        assert_eq!(p.feed("game.exe,1234,0x1,DXGI,0,0,1,0.51,0,0.1"), None);
        assert_eq!(p.feed("garbage"), None);
        let mut p2 = CsvParser::default();
        p2.feed("Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,CPUStartQPC,FrameTime,CPUBusy");
        assert_eq!(p2.feed("game.exe,1,0x1,DXGI,0,0,1,Hardware: Independent Flip,123,8.33,2"), Some(8.33));
    }

    #[test]
    fn downsampling_keeps_length_bound() {
        let v: Vec<f32> = (0..10_000).map(|i| i as f32).collect();
        let d = downsample(&v, 500);
        assert_eq!(d.len(), 500);
        assert!(d[0] < d[499]);
        assert_eq!(downsample(&v[..10], 500).len(), 10);
    }

    #[test]
    fn presentmon_path_validation() {
        assert!(validate_presentmon(Path::new("relative/PresentMon.exe")).is_err());
        assert!(validate_presentmon(Path::new("/tmp/notepad.exe")).is_err());
        assert!(validate_presentmon(Path::new("/tmp/PresentMon-2.0.exe")).is_err()); // doesn't exist
        assert!(Capture::start(Path::new("/tmp/x.exe"), "game.exe", "g").is_err());
    }

    #[test]
    fn failure_explanations() {
        assert!(explain_failure("error: Access is denied").contains("administrator"));
        assert!(explain_failure("").contains("exited"));
    }

    #[cfg(unix)]
    #[test]
    fn capture_with_a_stand_in_process_end_to_end() {
        // A tiny script named like PresentMon that prints a CSV, to exercise
        // spawning, parsing and finishing without Windows.
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let exe = d.path().join("PresentMon-test.exe");
        let mut s = String::from("#!/bin/sh\necho 'Application,MsBetweenPresents'\n");
        for _ in 0..200 {
            s.push_str("echo 'game.exe,8.0'\n");
        }
        s.push_str("sleep 5\n");
        std::fs::write(&exe, s).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut cap = Capture::start(&exe, "game.exe", "Game").unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let live = cap.live();
        assert!(live.frames >= 200, "{live:?}");
        assert!((live.current_fps.unwrap() - 125.0).abs() < 1e-6);
        let frames = cap.finish();
        let st = stats(&frames).unwrap();
        assert!((st.avg_fps - 125.0).abs() < 1e-6);
    }
}
