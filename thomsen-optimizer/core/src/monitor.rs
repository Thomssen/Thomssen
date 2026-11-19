//! Live resource monitoring. One persistent `sysinfo::System` is refreshed
//! incrementally (CPU, memory, disks, networks - never the full process list
//! on every tick), and the expensive GPU probes run on their own slower
//! cadence. Nothing is simulated: a metric Windows or the hardware does not
//! expose is reported as `None` and shown as "Unavailable" in the UI.

use crate::sys::proc_util::{command, run_capture};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuLive {
    pub name: Option<String>,
    /// 0-100, `None` when no counter is available.
    pub usage: Option<f32>,
    pub temperature_c: Option<f32>,
    pub vram_used_mb: Option<u64>,
    pub vram_total_mb: Option<u64>,
    /// Where the numbers came from ("NVIDIA driver (nvidia-smi)" / "Windows GPU Engine counters").
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub timestamp_ms: i64,
    pub cpu_usage: f32,
    pub cpu_temp_c: Option<f32>,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_available_bytes: u64,
    pub disk_read_bps: u64,
    pub disk_write_bps: u64,
    pub net_down_bps: u64,
    pub net_up_bps: u64,
    pub process_count: usize,
    pub uptime_secs: u64,
    pub gpu: Option<GpuLive>,
    /// This app's own footprint, so users can verify it stays light.
    pub self_cpu: f32,
    pub self_memory_bytes: u64,
}

pub struct Monitor {
    pub sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last: Instant,
    last_proc_refresh: Instant,
    process_count: usize,
    pub temps_enabled: bool,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let now = Instant::now();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last: now,
            last_proc_refresh: now - Duration::from_secs(60),
            process_count: 0,
            temps_enabled: true,
        }
    }

    /// Takes one sample (GPU data is filled in separately by [`GpuProbe`],
    /// so a slow helper process never holds up the rest of the app).
    pub fn sample(&mut self) -> Sample {
        let dt = self.last.elapsed().as_secs_f64().max(0.2);
        self.last = Instant::now();

        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.disks.refresh(true);
        self.networks.refresh(true);

        // The process *count* needs a list refresh; do it at most every 5 s.
        if self.last_proc_refresh.elapsed() > Duration::from_secs(5) {
            self.sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
            self.process_count = self.sys.processes().len();
            self.last_proc_refresh = Instant::now();
        } else {
            self.sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[Pid::from_u32(std::process::id())]), true, ProcessRefreshKind::nothing().with_cpu().with_memory());
        }

        let (mut rd, mut wr) = (0u64, 0u64);
        for d in self.disks.list() {
            let u = d.usage();
            rd += u.read_bytes;
            wr += u.written_bytes;
        }
        let (mut down, mut up) = (0u64, 0u64);
        for (_, n) in self.networks.iter() {
            down += n.received();
            up += n.transmitted();
        }

        let cpu_temp = if self.temps_enabled { self.cpu_temperature() } else { None };
        let me = self.sys.process(Pid::from_u32(std::process::id()));
        let cores = self.sys.cpus().len().max(1) as f32;
        Sample {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            cpu_usage: self.sys.global_cpu_usage(),
            cpu_temp_c: cpu_temp,
            memory_used_bytes: self.sys.used_memory(),
            memory_total_bytes: self.sys.total_memory(),
            memory_available_bytes: self.sys.available_memory(),
            disk_read_bps: (rd as f64 / dt) as u64,
            disk_write_bps: (wr as f64 / dt) as u64,
            net_down_bps: (down as f64 / dt) as u64,
            net_up_bps: (up as f64 / dt) as u64,
            process_count: self.process_count,
            uptime_secs: System::uptime(),
            gpu: None,
            self_cpu: me.map(|p| p.cpu_usage() / cores).unwrap_or(0.0),
            self_memory_bytes: me.map(|p| p.memory()).unwrap_or(0),
        }
    }

    fn cpu_temperature(&mut self) -> Option<f32> {
        self.components.refresh(true);
        self.components
            .iter()
            .filter(|c| {
                let l = c.label().to_ascii_lowercase();
                l.contains("cpu") || l.contains("package") || l.contains("tctl") || l.contains("core")
            })
            .filter_map(|c| c.temperature())
            .filter(|t| t.is_finite() && *t > 0.0 && *t < 125.0)
            .reduce(f32::max)
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU readings, throttled and kept apart from [`Monitor`] (it may spawn a
/// helper process). NVIDIA's own tool is preferred; otherwise the Windows GPU
/// Engine counters are used, at a slower minimum cadence because reading them
/// is comparatively expensive.
#[derive(Default)]
pub struct GpuProbe {
    last: Option<(Instant, Option<GpuLive>)>,
    nvidia_missing: bool,
}

impl GpuProbe {
    pub fn get(&mut self, every: Duration) -> Option<GpuLive> {
        let every = if self.nvidia_missing { every.max(Duration::from_secs(5)) } else { every };
        if let Some((t, v)) = &self.last {
            if t.elapsed() < every {
                return v.clone();
            }
        }
        let v = self.probe();
        self.last = Some((Instant::now(), v.clone()));
        v
    }

    pub fn cached(&self) -> Option<GpuLive> {
        self.last.as_ref().and_then(|l| l.1.clone())
    }

    fn probe(&mut self) -> Option<GpuLive> {
        if !self.nvidia_missing {
            match nvidia_smi() {
                Some(g) => return Some(g),
                None => self.nvidia_missing = true,
            }
        }
        windows_gpu_counters()
    }
}

fn nvidia_smi() -> Option<GpuLive> {
    let mut c = command("nvidia-smi");
    c.args(["--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"]);
    let out = run_capture(c, Duration::from_secs(4)).ok()?;
    parse_nvidia_smi(&out)
}

pub fn parse_nvidia_smi(out: &str) -> Option<GpuLive> {
    let line = out.lines().find(|l| !l.trim().is_empty())?;
    let f: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if f.len() < 5 {
        return None;
    }
    let num = |s: &str| s.parse::<f32>().ok();
    Some(GpuLive {
        name: Some(f[0].to_string()),
        usage: num(f[1]),
        temperature_c: num(f[2]),
        vram_used_mb: num(f[3]).map(|v| v as u64),
        vram_total_mb: num(f[4]).map(|v| v as u64),
        source: Some("NVIDIA driver (nvidia-smi)".into()),
    })
}

/// Vendor-neutral fallback: the same "GPU Engine" performance counters Task
/// Manager uses, read once via `typeperf`.
fn windows_gpu_counters() -> Option<GpuLive> {
    #[cfg(not(windows))]
    {
        None
    }
    #[cfg(windows)]
    {
        let mut c = command("typeperf.exe");
        c.args(["\\GPU Engine(*)\\Utilization Percentage", "-sc", "1"]);
        let out = run_capture(c, Duration::from_secs(6)).ok()?;
        let usage = parse_typeperf_gpu(&out)?;
        let mut m = command("typeperf.exe");
        m.args(["\\GPU Adapter Memory(*)\\Dedicated Usage", "-sc", "1"]);
        let vram_used = run_capture(m, Duration::from_secs(6)).ok().and_then(|o| parse_typeperf_sum(&o)).map(|b| (b / (1024.0 * 1024.0)) as u64);
        Some(GpuLive { name: None, usage: Some(usage), temperature_c: None, vram_used_mb: vram_used, vram_total_mb: None, source: Some("Windows GPU Engine counters".into()) })
    }
}

fn split_csv_line(l: &str) -> Vec<String> {
    l.split("\",\"").map(|s| s.trim_matches('"').to_string()).collect()
}

fn typeperf_rows(out: &str) -> Option<(Vec<String>, Vec<String>)> {
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with('"')).collect();
    if lines.len() < 2 {
        return None;
    }
    Some((split_csv_line(lines[0]), split_csv_line(lines[1])))
}

/// Task-Manager style: per engine type sum the processes' utilisation, then
/// take the busiest engine type per adapter, then the busiest adapter.
pub fn parse_typeperf_gpu(out: &str) -> Option<f32> {
    let (head, vals) = typeperf_rows(out)?;
    let mut per: std::collections::HashMap<(String, String), f32> = std::collections::HashMap::new();
    for (h, v) in head.iter().zip(vals.iter()).skip(1) {
        let Ok(v) = v.trim().parse::<f32>() else { continue };
        let (Some(l), Some(e)) = (h.find("luid_"), h.find("engtype_")) else { continue };
        let luid_full = h[l..].split("_phys").next().unwrap_or("").to_string();
        let etype = h[e..].split(')').next().unwrap_or("").to_string();
        *per.entry((luid_full, etype)).or_insert(0.0) += v;
    }
    per.values().cloned().reduce(f32::max).map(|v| v.clamp(0.0, 100.0))
}

pub fn parse_typeperf_sum(out: &str) -> Option<f64> {
    let (_, vals) = typeperf_rows(out)?;
    let s: f64 = vals.iter().skip(1).filter_map(|v| v.trim().parse::<f64>().ok()).sum();
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_smi_parsing() {
        let g = parse_nvidia_smi("NVIDIA GeForce RTX 4070, 37, 55, 2048, 12282\n").unwrap();
        assert_eq!(g.name.as_deref(), Some("NVIDIA GeForce RTX 4070"));
        assert_eq!(g.usage, Some(37.0));
        assert_eq!(g.temperature_c, Some(55.0));
        assert_eq!(g.vram_total_mb, Some(12282));
        assert!(parse_nvidia_smi("").is_none());
        // Unsupported fields ("[N/A]") become None instead of fake zeros.
        let g = parse_nvidia_smi("GPU, [N/A], [N/A], 1, 2").unwrap();
        assert_eq!(g.usage, None);
    }

    #[test]
    fn typeperf_gpu_parsing() {
        let out = "\"(PDH-CSV 4.0)\",\"\\\\PC\\GPU Engine(pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D)\\Utilization Percentage\",\"\\\\PC\\GPU Engine(pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D)\\Utilization Percentage\",\"\\\\PC\\GPU Engine(pid_2_luid_0x0_0x1_phys_0_eng_1_engtype_Copy)\\Utilization Percentage\"\n\"01/01/2026 10:00:00.000\",\"20.5\",\"30.5\",\"60\"\n\nExiting, please wait...";
        // 3D: 20.5+30.5 = 51; Copy: 60 -> busiest engine type wins.
        assert_eq!(parse_typeperf_gpu(out), Some(60.0));
        assert!(parse_typeperf_gpu("garbage").is_none());
    }

    #[test]
    fn sample_has_real_values() {
        let mut m = Monitor::new();
        std::thread::sleep(Duration::from_millis(250));
        let s = m.sample();
        assert!(s.memory_total_bytes > 0);
        assert!(s.process_count > 0);
        assert!(s.self_memory_bytes > 0);
    }
}
