//! The boundary between platform-independent logic and Windows itself.
//!
//! Everything that reads or changes Windows state (registry, power plans,
//! window enumeration, the recycle bin ...) goes through [`SystemOps`] or the
//! functions in [`win`]. That keeps the optimization engine, the restore
//! journal and the analysers unit-testable with [`fake::FakeOps`], and means
//! the only `unsafe` / Windows-specific code lives in `sys/windows.rs`.

pub mod fake;
pub mod proc_util;
pub mod win;
#[cfg(windows)]
pub mod windows;

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum RegData {
    Dword(u32),
    Qword(u64),
    Sz(String),
    Binary(Vec<u8>),
}

impl RegData {
    pub fn as_dword(&self) -> Option<u32> {
        if let RegData::Dword(v) = self { Some(*v) } else { None }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let RegData::Sz(v) = self { Some(v) } else { None }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlanKind {
    PowerSaver,
    Balanced,
    HighPerformance,
    Ultimate,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PowerPlan {
    pub guid: String,
    pub name: String,
    pub active: bool,
    pub kind: PlanKind,
}

pub const GUID_BALANCED: &str = "381b4222-f694-41f0-9685-ff5bb260df2e";
pub const GUID_HIGH: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const GUID_SAVER: &str = "a1841308-3541-4fab-bc81-f71556f20b4a";
pub const GUID_ULTIMATE_TEMPLATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
/// Marker appended to the name of the Ultimate Performance plan we create, so
/// it can be recognised again regardless of the Windows display language.
pub const ULTIMATE_MARKER: &str = "(Thomsen)";

pub fn plan_kind(guid: &str, name: &str) -> PlanKind {
    let g = guid.to_ascii_lowercase();
    if g == GUID_BALANCED {
        PlanKind::Balanced
    } else if g == GUID_HIGH {
        PlanKind::HighPerformance
    } else if g == GUID_SAVER {
        PlanKind::PowerSaver
    } else if g == GUID_ULTIMATE_TEMPLATE || name.contains(ULTIMATE_MARKER) || name.to_ascii_lowercase().contains("ultimate") {
        PlanKind::Ultimate
    } else {
        PlanKind::Custom
    }
}

/// Strict 8-4-4-4-12 hex GUID check. Anything that reaches `powercfg` as an
/// argument must pass this first - no user text is ever interpolated into a
/// command line.
pub fn valid_guid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let lens = [8usize, 4, 4, 4, 12];
    parts.len() == 5 && parts.iter().zip(lens).all(|(p, l)| p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit()))
}

pub trait SystemOps: Send + Sync {
    fn reg_get(&self, hive: Hive, path: &str, name: &str) -> Result<Option<RegData>>;
    fn reg_set(&self, hive: Hive, path: &str, name: &str, data: &RegData) -> Result<()>;
    fn reg_delete_value(&self, hive: Hive, path: &str, name: &str) -> Result<()>;
    fn reg_values(&self, hive: Hive, path: &str) -> Result<Vec<(String, RegData)>>;
    fn reg_subkeys(&self, hive: Hive, path: &str) -> Result<Vec<String>>;

    fn power_plans(&self) -> Result<Vec<PowerPlan>>;
    fn power_set_active(&self, guid: &str) -> Result<()>;
    /// Creates (or finds) the Ultimate Performance plan.
    fn power_ensure_ultimate(&self) -> Result<PowerPlan>;
}

/// The real implementation on Windows; an "unsupported" stub elsewhere so the
/// crate (and its tests) still build on any development machine.
pub fn platform_ops() -> Arc<dyn SystemOps> {
    #[cfg(windows)]
    {
        Arc::new(windows::WindowsOps)
    }
    #[cfg(not(windows))]
    {
        Arc::new(UnsupportedOps)
    }
}

#[cfg(not(windows))]
pub struct UnsupportedOps;

#[cfg(not(windows))]
impl SystemOps for UnsupportedOps {
    fn reg_get(&self, _: Hive, _: &str, _: &str) -> Result<Option<RegData>> {
        Err(unsupported())
    }
    fn reg_set(&self, _: Hive, _: &str, _: &str, _: &RegData) -> Result<()> {
        Err(unsupported())
    }
    fn reg_delete_value(&self, _: Hive, _: &str, _: &str) -> Result<()> {
        Err(unsupported())
    }
    fn reg_values(&self, _: Hive, _: &str) -> Result<Vec<(String, RegData)>> {
        Err(unsupported())
    }
    fn reg_subkeys(&self, _: Hive, _: &str) -> Result<Vec<String>> {
        Err(unsupported())
    }
    fn power_plans(&self) -> Result<Vec<PowerPlan>> {
        Err(unsupported())
    }
    fn power_set_active(&self, _: &str) -> Result<()> {
        Err(unsupported())
    }
    fn power_ensure_ultimate(&self) -> Result<PowerPlan> {
        Err(unsupported())
    }
}

#[cfg(not(windows))]
fn unsupported() -> Error {
    Error::Unsupported("This feature is only available on Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_validation() {
        assert!(valid_guid(GUID_HIGH));
        assert!(!valid_guid("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c & calc"));
        assert!(!valid_guid(""));
        assert!(!valid_guid("zzzzzzzz-e8bf-4a96-9a85-a6e23a8c635c"));
    }

    #[test]
    fn plan_kinds() {
        assert_eq!(plan_kind(GUID_BALANCED, "x"), PlanKind::Balanced);
        assert_eq!(plan_kind("1111", "Ultimate Performance"), PlanKind::Ultimate);
        assert_eq!(plan_kind("1111", "Mine"), PlanKind::Custom);
    }
}
