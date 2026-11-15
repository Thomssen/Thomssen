//! Secret storage. On Windows this is Windows Credential Manager. On other platforms
//! (development only) a 0600 file in the data directory is used.

const SERVICE: &str = "PSYCHOSEC";

#[cfg(windows)]
pub fn get(name: &str) -> Option<String> {
    keyring::Entry::new(SERVICE, name).ok()?.get_password().ok()
}
#[cfg(windows)]
pub fn set(name: &str, value: &str) -> Result<(), String> {
    keyring::Entry::new(SERVICE, name)
        .and_then(|e| e.set_password(value))
        .map_err(|e| e.to_string())
}
#[cfg(windows)]
pub fn delete(name: &str) {
    if let Ok(e) = keyring::Entry::new(SERVICE, name) {
        let _ = e.delete_credential();
    }
}

#[cfg(not(windows))]
fn path(name: &str) -> std::path::PathBuf {
    crate::config::data_dir().join(format!(".cred-{}-{}", SERVICE.replace(' ', "_"), name))
}
#[cfg(not(windows))]
pub fn get(name: &str) -> Option<String> {
    std::fs::read_to_string(path(name)).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}
#[cfg(not(windows))]
pub fn set(name: &str, value: &str) -> Result<(), String> {
    let p = path(name);
    std::fs::write(&p, value).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn delete(name: &str) {
    let _ = std::fs::remove_file(path(name));
}

pub const SESSION_TOKEN: &str = "session-token";
pub const DB_KEY: &str = "case-db-key";
