//! A stable-but-anonymous per-PC identifier for "one active PC at a time"
//! license enforcement - not aggressive hardware fingerprinting (no CPU/
//! disk serials, no MAC address), just the one GUID Windows itself
//! generates once at install time and never changes on its own. Hashed
//! before it ever leaves this PC, so the server never sees the raw GUID.

use sha2::{Digest, Sha256};

#[cfg(target_os = "windows")]
fn read_machine_guid() -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_VALUE_TYPE};

    unsafe {
        let mut hkey = Default::default();
        let subkey: Vec<u16> = "SOFTWARE\\Microsoft\\Cryptography\0".encode_utf16().collect();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, PCWSTR(subkey.as_ptr()), 0, KEY_READ, &mut hkey).is_err() {
            return None;
        }

        let value_name: Vec<u16> = "MachineGuid\0".encode_utf16().collect();
        let mut buf = [0u16; 64];
        let mut buf_size: u32 = (buf.len() * 2) as u32;
        let mut value_type = REG_VALUE_TYPE::default();

        let status = RegQueryValueExW(
            hkey,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(buf.as_mut_ptr() as *mut u8),
            Some(&mut buf_size),
        );
        let _ = RegCloseKey(hkey);

        if status != ERROR_SUCCESS {
            return None;
        }
        let len_u16 = (buf_size as usize / 2).saturating_sub(1); // drop the trailing NUL
        let guid = String::from_utf16_lossy(&buf[..len_u16.min(buf.len())]);
        if guid.is_empty() {
            None
        } else {
            Some(guid)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn read_machine_guid() -> Option<String> {
    None
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Stable across app reinstalls and updates (tied to the Windows install,
/// not to Thomsen Shield's own data). Falls back to a random id, persisted
/// locally, only in the unlikely case the registry read fails - still
/// stable across restarts, just not tied to the OS install itself.
pub fn stable_device_id() -> String {
    let source = read_machine_guid().unwrap_or_else(fallback_persisted_id);
    let digest = Sha256::digest(source.as_bytes());
    hex_encode(&digest)[..32].to_string()
}

fn fallback_persisted_id() -> String {
    let Ok(dir) = crate::store::paths::data_dir() else {
        return uuid::Uuid::new_v4().to_string();
    };
    let path = dir.join("device-id-fallback");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        if !existing.trim().is_empty() {
            return existing;
        }
    }
    let generated = uuid::Uuid::new_v4().to_string();
    let _ = std::fs::write(&path, &generated);
    generated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encode_matches_known_vector() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0x10]), "00ff10");
    }
}
