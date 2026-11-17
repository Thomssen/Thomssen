//! Windows DPAPI (CryptProtectData) wrapper. Data is bound to the current Windows user.

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};

const ENTROPY: &[u8] = b"PrivacyBlur/custom-entries/v1";

fn blob(d: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: d.len() as u32, pbData: d.as_ptr() as *mut u8 }
}

unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
    let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
    v
}

pub fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = blob(plain);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(&input, w!("Privacy Blur"), Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
            .map_err(|e| format!("DPAPI encrypt failed: {e}"))?;
        Ok(take(out))
    }
}

pub fn unprotect(cipher: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let input = blob(cipher);
        let entropy = blob(ENTROPY);
        let mut out = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(&input, None::<*mut PWSTR>, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
            .map_err(|e| format!("DPAPI decrypt failed (different Windows user or damaged file): {e}"))?;
        Ok(take(out))
    }
}
