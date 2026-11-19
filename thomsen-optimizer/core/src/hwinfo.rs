//! Static hardware / OS facts for the Dashboard and System Information page.
//! Sources: the Windows registry (CPU, GPU, OS), `sysinfo` (cores, memory,
//! drives) and one read-only, fixed PowerShell CIM query (RAM speed, physical
//! disk health). Anything unavailable stays `None` - nothing is guessed.

use crate::sys::proc_util::powershell;
use crate::sys::{Hive, RegData, SystemOps};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{DiskKind, Disks, System};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub name: String,
    pub vendor: String,
    pub physical_cores: Option<usize>,
    pub logical_processors: usize,
    pub base_mhz: Option<u32>,
    pub current_mhz: Option<u64>,
    pub architecture: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub manufacturer: Option<String>,
    pub vram_bytes: Option<u64>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub speed_mhz: Option<u32>,
    pub modules: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub name: String,
    pub mount: String,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub kind: String,
    pub removable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalDisk {
    pub name: String,
    pub media_type: Option<String>,
    pub bus_type: Option<String>,
    pub health: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub build: String,
    pub architecture: String,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub memory: MemoryInfo,
    pub drives: Vec<DriveInfo>,
    pub physical_disks: Vec<PhysicalDisk>,
    pub os: OsInfo,
}

const CV: &str = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
const GPU_CLASS: &str = "SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11cd-bfc1-08002be10318}";

fn reg_str(ops: &dyn SystemOps, hive: Hive, path: &str, name: &str) -> Option<String> {
    ops.reg_get(hive, path, name).ok().flatten().and_then(|d| d.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty())
}

fn reg_u64(d: Option<RegData>) -> Option<u64> {
    match d? {
        RegData::Qword(v) => Some(v),
        RegData::Dword(v) => Some(v as u64),
        RegData::Binary(b) if b.len() >= 8 => Some(u64::from_le_bytes(b[..8].try_into().ok()?)),
        RegData::Binary(b) if b.len() >= 4 => Some(u32::from_le_bytes(b[..4].try_into().ok()?) as u64),
        _ => None,
    }
}

pub fn gpus_from_registry(ops: &dyn SystemOps) -> Vec<GpuInfo> {
    let Ok(keys) = ops.reg_subkeys(Hive::LocalMachine, GPU_CLASS) else { return vec![] };
    let mut out = Vec::new();
    for k in keys.iter().filter(|k| k.len() == 4 && k.chars().all(|c| c.is_ascii_digit())) {
        let path = format!("{GPU_CLASS}\\{k}");
        let Some(name) = reg_str(ops, Hive::LocalMachine, &path, "DriverDesc") else { continue };
        if name.to_ascii_lowercase().contains("basic display") || name.to_ascii_lowercase().contains("remote display") {
            continue;
        }
        let vram = reg_u64(ops.reg_get(Hive::LocalMachine, &path, "HardwareInformation.qwMemorySize").ok().flatten())
            .or_else(|| reg_u64(ops.reg_get(Hive::LocalMachine, &path, "HardwareInformation.MemorySize").ok().flatten()));
        out.push(GpuInfo { name, manufacturer: reg_str(ops, Hive::LocalMachine, &path, "ProviderName"), vram_bytes: vram, driver_version: reg_str(ops, Hive::LocalMachine, &path, "DriverVersion") });
    }
    out
}

pub fn friendly_windows_name(product: &str, build: u32) -> String {
    // The registry ProductName still says "Windows 10" on Windows 11.
    if build >= 22000 {
        product.replace("Windows 10", "Windows 11")
    } else {
        product.to_string()
    }
}

#[derive(Default)]
pub struct CimData {
    pub ram_speed_mhz: Option<u32>,
    pub ram_modules: Option<usize>,
    pub disks: Vec<PhysicalDisk>,
}

fn as_list(v: Option<&serde_json::Value>) -> Vec<&serde_json::Value> {
    match v {
        Some(serde_json::Value::Array(a)) => a.iter().collect(),
        Some(o @ serde_json::Value::Object(_)) => vec![o],
        _ => vec![],
    }
}

pub fn parse_cim(json: &str) -> CimData {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json.trim()) else { return CimData::default() };
    let ram = as_list(v.get("ram"));
    let speed = ram.iter().filter_map(|m| m.get("ConfiguredClockSpeed").and_then(|x| x.as_u64()).filter(|s| *s > 0).or_else(|| m.get("Speed").and_then(|x| x.as_u64()))).max().map(|s| s as u32);
    let disks = as_list(v.get("disks"))
        .into_iter()
        .map(|d| {
            let s = |k: &str| d.get(k).and_then(|x| x.as_str()).map(|s| s.to_string()).filter(|s| !s.is_empty() && s != "Unspecified");
            PhysicalDisk { name: s("FriendlyName").unwrap_or_else(|| "Disk".into()), media_type: s("MediaType"), bus_type: s("BusType"), health: s("HealthStatus"), size_bytes: d.get("Size").and_then(|x| x.as_u64()) }
        })
        .collect();
    CimData { ram_speed_mhz: speed, ram_modules: (!ram.is_empty()).then_some(ram.len()), disks }
}

fn query_cim() -> CimData {
    let script = "$ErrorActionPreference='SilentlyContinue'; @{ ram = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Speed,ConfiguredClockSpeed); disks = @(Get-PhysicalDisk | Select-Object FriendlyName,MediaType,HealthStatus,BusType,Size) } | ConvertTo-Json -Compress -Depth 4";
    #[cfg(windows)]
    {
        powershell(script, Duration::from_secs(20)).map(|o| parse_cim(&o)).unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        let _ = (script, powershell as fn(&str, Duration) -> crate::error::Result<String>);
        CimData::default()
    }
}

pub fn collect(ops: &dyn SystemOps, sys: &System) -> SystemInfo {
    let cpus = sys.cpus();
    let cpu_key = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0";
    let name = reg_str(ops, Hive::LocalMachine, cpu_key, "ProcessorNameString").or_else(|| cpus.first().map(|c| c.brand().trim().to_string())).unwrap_or_else(|| "Unknown processor".into());
    let cpu = CpuInfo {
        name,
        vendor: cpus.first().map(|c| c.vendor_id().to_string()).unwrap_or_default(),
        physical_cores: sys.physical_core_count(),
        logical_processors: cpus.len(),
        base_mhz: ops.reg_get(Hive::LocalMachine, cpu_key, "~MHz").ok().flatten().and_then(|d| d.as_dword()),
        current_mhz: cpus.iter().map(|c| c.frequency()).max().filter(|f| *f > 0),
        architecture: System::cpu_arch(),
    };

    let build_str = reg_str(ops, Hive::LocalMachine, CV, "CurrentBuild").or_else(System::kernel_version).unwrap_or_default();
    let build_num: u32 = build_str.parse().unwrap_or(0);
    let ubr = ops.reg_get(Hive::LocalMachine, CV, "UBR").ok().flatten().and_then(|d| d.as_dword());
    let product = reg_str(ops, Hive::LocalMachine, CV, "ProductName").or_else(System::long_os_version).unwrap_or_else(|| "Windows".into());
    let os = OsInfo {
        name: friendly_windows_name(&product, build_num),
        version: reg_str(ops, Hive::LocalMachine, CV, "DisplayVersion").or_else(|| reg_str(ops, Hive::LocalMachine, CV, "ReleaseId")).or_else(System::os_version).unwrap_or_default(),
        build: match ubr {
            Some(u) => format!("{build_str}.{u}"),
            None => build_str,
        },
        architecture: System::cpu_arch(),
        uptime_secs: System::uptime(),
    };

    let cim = query_cim();
    let drives = Disks::new_with_refreshed_list()
        .list()
        .iter()
        .map(|d| DriveInfo {
            name: d.name().to_string_lossy().into_owned(),
            mount: d.mount_point().to_string_lossy().into_owned(),
            file_system: d.file_system().to_string_lossy().into_owned(),
            total_bytes: d.total_space(),
            free_bytes: d.available_space(),
            kind: match d.kind() {
                DiskKind::SSD => "SSD".into(),
                DiskKind::HDD => "HDD".into(),
                _ => "Unknown".into(),
            },
            removable: d.is_removable(),
        })
        .collect();

    SystemInfo {
        cpu,
        gpus: gpus_from_registry(ops),
        memory: MemoryInfo { total_bytes: sys.total_memory(), speed_mhz: cim.ram_speed_mhz, modules: cim.ram_modules },
        drives,
        physical_disks: cim.disks,
        os,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cim_json_for_single_and_multiple_items() {
        let c = parse_cim(r#"{"ram":[{"Speed":3200,"ConfiguredClockSpeed":3600},{"Speed":3200,"ConfiguredClockSpeed":3600}],"disks":{"FriendlyName":"Samsung SSD","MediaType":"SSD","HealthStatus":"Healthy","BusType":"NVMe","Size":1000204886016}}"#);
        assert_eq!(c.ram_speed_mhz, Some(3600));
        assert_eq!(c.ram_modules, Some(2));
        assert_eq!(c.disks.len(), 1);
        assert_eq!(c.disks[0].health.as_deref(), Some("Healthy"));
        assert!(parse_cim("not json").disks.is_empty());
    }

    #[test]
    fn windows_11_name_fix() {
        assert_eq!(friendly_windows_name("Windows 10 Pro", 22631), "Windows 11 Pro");
        assert_eq!(friendly_windows_name("Windows 10 Pro", 19045), "Windows 10 Pro");
    }

    #[test]
    fn vram_from_binary_registry_value() {
        assert_eq!(reg_u64(Some(RegData::Binary(vec![0, 0, 0, 0x80, 2, 0, 0, 0]))), Some(0x2_8000_0000));
    }
}
