//! Real Windows implementation of [`SystemOps`]: registry via `winreg`,
//! power plans via the built-in `powercfg.exe`.

use super::proc_util::{command, run_capture};
use super::*;
use std::time::Duration;
use winreg::enums::*;
use winreg::{RegKey, RegValue};

pub struct WindowsOps;

fn root(h: Hive) -> RegKey {
    RegKey::predef(match h {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    })
}

fn map_io(e: std::io::Error, what: &str) -> Error {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        Error::PermissionDenied(format!("Windows denied access to {what}. Administrator rights are required for this change."))
    } else {
        Error::Io(e)
    }
}

fn from_raw(v: RegValue) -> Option<RegData> {
    match v.vtype {
        REG_DWORD if v.bytes.len() >= 4 => Some(RegData::Dword(u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]]))),
        REG_QWORD if v.bytes.len() >= 8 => {
            let mut b = [0u8; 8];
            b.copy_from_slice(&v.bytes[..8]);
            Some(RegData::Qword(u64::from_le_bytes(b)))
        }
        REG_SZ | REG_EXPAND_SZ => {
            let u: Vec<u16> = v.bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let end = u.iter().position(|&c| c == 0).unwrap_or(u.len());
            Some(RegData::Sz(String::from_utf16_lossy(&u[..end])))
        }
        REG_BINARY => Some(RegData::Binary(v.bytes)),
        _ => None,
    }
}

fn to_raw(d: &RegData) -> RegValue {
    match d {
        RegData::Dword(v) => RegValue { bytes: v.to_le_bytes().to_vec(), vtype: REG_DWORD },
        RegData::Qword(v) => RegValue { bytes: v.to_le_bytes().to_vec(), vtype: REG_QWORD },
        RegData::Sz(s) => {
            let mut bytes: Vec<u8> = s.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect();
            bytes.shrink_to_fit();
            RegValue { bytes, vtype: REG_SZ }
        }
        RegData::Binary(b) => RegValue { bytes: b.clone(), vtype: REG_BINARY },
    }
}

impl SystemOps for WindowsOps {
    fn reg_get(&self, hive: Hive, path: &str, name: &str) -> Result<Option<RegData>> {
        let key = match root(hive).open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(map_io(e, path)),
        };
        match key.get_raw_value(name) {
            Ok(v) => Ok(from_raw(v)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(map_io(e, path)),
        }
    }

    fn reg_set(&self, hive: Hive, path: &str, name: &str, data: &RegData) -> Result<()> {
        let (key, _) = root(hive).create_subkey_with_flags(path, KEY_WRITE | KEY_READ).map_err(|e| map_io(e, path))?;
        key.set_raw_value(name, &to_raw(data)).map_err(|e| map_io(e, path))
    }

    fn reg_delete_value(&self, hive: Hive, path: &str, name: &str) -> Result<()> {
        let key = match root(hive).open_subkey_with_flags(path, KEY_WRITE) {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(map_io(e, path)),
        };
        match key.delete_value(name) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(map_io(e, path)),
        }
    }

    fn reg_values(&self, hive: Hive, path: &str) -> Result<Vec<(String, RegData)>> {
        let key = match root(hive).open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(map_io(e, path)),
        };
        Ok(key.enum_values().filter_map(|r| r.ok()).filter_map(|(n, v)| from_raw(v).map(|d| (n, d))).collect())
    }

    fn reg_subkeys(&self, hive: Hive, path: &str) -> Result<Vec<String>> {
        let key = match root(hive).open_subkey_with_flags(path, KEY_READ) {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(map_io(e, path)),
        };
        Ok(key.enum_keys().filter_map(|r| r.ok()).collect())
    }

    fn power_plans(&self) -> Result<Vec<PowerPlan>> {
        let mut c = command("powercfg.exe");
        c.arg("/list");
        let out = run_capture(c, Duration::from_secs(10))?;
        Ok(parse_powercfg_list(&out))
    }

    fn power_set_active(&self, guid: &str) -> Result<()> {
        if !valid_guid(guid) {
            return Err(Error::Invalid("not a valid power plan GUID".into()));
        }
        let mut c = command("powercfg.exe");
        c.args(["/setactive", guid]);
        run_capture(c, Duration::from_secs(10)).map(|_| ())
    }

    fn power_ensure_ultimate(&self) -> Result<PowerPlan> {
        if let Some(p) = self.power_plans()?.into_iter().find(|p| p.name.contains(ULTIMATE_MARKER) || p.kind == PlanKind::Ultimate) {
            return Ok(p);
        }
        let mut c = command("powercfg.exe");
        c.args(["/duplicatescheme", GUID_ULTIMATE_TEMPLATE]);
        let out = run_capture(c, Duration::from_secs(10))
            .map_err(|_| Error::Unsupported("The Ultimate Performance plan is not available on this PC.".into()))?;
        let guid = find_guid(&out).ok_or_else(|| Error::Command("powercfg did not return the new plan".into()))?;
        let name = format!("Ultimate Performance {ULTIMATE_MARKER}");
        let mut r = command("powercfg.exe");
        r.args(["/changename", &guid, &name]);
        let _ = run_capture(r, Duration::from_secs(10));
        Ok(PowerPlan { guid, name, active: false, kind: PlanKind::Ultimate })
    }
}

/// First GUID-shaped token in `s`.
pub fn find_guid(s: &str) -> Option<String> {
    let b = s.as_bytes();
    if b.len() < 36 {
        return None;
    }
    (0..=b.len() - 36).find_map(|i| {
        let cand = s.get(i..i + 36)?;
        valid_guid(cand).then(|| cand.to_ascii_lowercase())
    })
}

/// Locale-independent parse of `powercfg /list` (the "Power Scheme GUID:"
/// label and the plan names are localised; the GUID, parentheses and trailing
/// `*` are not).
pub fn parse_powercfg_list(out: &str) -> Vec<PowerPlan> {
    out.lines()
        .filter_map(|line| {
            let guid = find_guid(line)?;
            let name = match (line.find('('), line.rfind(')')) {
                (Some(a), Some(b)) if b > a => line[a + 1..b].trim().to_string(),
                _ => guid.clone(),
            };
            let active = line.trim_end().ends_with('*');
            let kind = plan_kind(&guid, &name);
            Some(PowerPlan { guid, name, active, kind })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powercfg_output() {
        let out = "Existing Power Schemes (* Active)\n-----------------------------------\nPower Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced) *\nPower Scheme GUID: 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c  (High performance)\n";
        let p = parse_powercfg_list(out);
        assert_eq!(p.len(), 2);
        assert!(p[0].active && p[0].kind == PlanKind::Balanced);
        assert_eq!(p[1].name, "High performance");
        assert!(!p[1].active);
    }
}
