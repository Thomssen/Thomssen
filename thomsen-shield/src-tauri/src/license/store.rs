//! Persists the signed activation token via Windows DPAPI
//! (`CryptProtectData`/`CryptUnprotectData`), scoped to the current user by
//! default - the file on disk is useless copied to another machine or read
//! by another account. This is *storage* encryption only; the token's
//! authenticity comes from the Ed25519 signature in `license::token`, not
//! from DPAPI.

use windows::Win32::Foundation::LocalFree;
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

use crate::error::{AppError, AppResult};
use crate::store::paths;

use super::token::SignedToken;

fn dpapi_protect(data: &[u8]) -> AppResult<Vec<u8>> {
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(&mut input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|e| AppError::new(format!("Couldn't securely save the license: {e}")))?;
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}

fn dpapi_unprotect(data: &[u8]) -> AppResult<Vec<u8>> {
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(&mut input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|e| AppError::new(format!("Couldn't read the saved license: {e}")))?;
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as *mut _));
        Ok(result)
    }
}

pub fn save_token(token: &SignedToken) -> AppResult<()> {
    let json = serde_json::to_vec(token)?;
    let encrypted = dpapi_protect(&json)?;
    std::fs::write(paths::license_cache_path()?, encrypted)?;
    Ok(())
}

pub fn load_token() -> AppResult<Option<SignedToken>> {
    let path = paths::license_cache_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let encrypted = std::fs::read(&path)?;
    if encrypted.is_empty() {
        return Ok(None);
    }
    let json = dpapi_unprotect(&encrypted)?;
    Ok(Some(serde_json::from_slice(&json)?))
}

pub fn clear_token() -> AppResult<()> {
    let path = paths::license_cache_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
