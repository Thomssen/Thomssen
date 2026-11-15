//! Verified update flow: signed release manifest (Ed25519) + SHA-256 of the installer.
use crate::api::Release;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// The exact byte string that the release signing key signs. Must match
/// `scripts/release-sign.mjs` and `services/api/src/lib/release.ts`.
pub fn signed_message(r: &Release) -> String {
    format!("psychosec-release\n{}\n{}\n{}", r.version, r.download_url, r.checksum_sha256.to_ascii_lowercase())
}

pub fn verify_manifest(r: &Release, pubkey_b64: &str) -> Result<(), String> {
    let sig_b64 = r.signature.as_deref().ok_or("release manifest is not signed")?;
    let key_bytes: [u8; 32] = B64
        .decode(pubkey_b64.trim())
        .map_err(|_| "embedded update key is invalid")?
        .try_into()
        .map_err(|_| "embedded update key has the wrong length")?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| "embedded update key is invalid")?;
    let sig_bytes: [u8; 64] = B64
        .decode(sig_b64.trim())
        .map_err(|_| "signature is not valid base64")?
        .try_into()
        .map_err(|_| "signature has the wrong length")?;
    key.verify(signed_message(r).as_bytes(), &Signature::from_bytes(&sig_bytes))
        .map_err(|_| "release signature verification FAILED".to_string())
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (semver::Version::parse(candidate), semver::Version::parse(current)) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    }
}

fn check_url(r: &Release) -> Result<url::Url, String> {
    let u = url::Url::parse(&r.download_url).map_err(|_| "invalid download URL")?;
    if u.scheme() != "https" {
        return Err("download URL is not HTTPS".into());
    }
    if r.checksum_sha256.len() != 64 || !r.checksum_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid checksum in release metadata".into());
    }
    Ok(u)
}

/// Validate a release and decide whether it is acceptable. Refuses unsigned releases
/// or when this build has no update key embedded.
pub fn validate(r: &Release) -> Result<(), String> {
    check_url(r)?;
    let key = crate::config::update_pubkey().ok_or("this build has no update signing key; updates are disabled")?;
    verify_manifest(r, key)
}

pub async fn download(r: &Release, tx: impl Fn(u64, Option<u64>)) -> Result<PathBuf, String> {
    validate(r)?;
    let url = check_url(r)?;
    let client = reqwest::Client::builder()
        .user_agent(format!("Psychosec/{}", crate::config::VERSION))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(url).send().await.map_err(|e| e.without_url().to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length();
    let dir = crate::config::data_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("Psychosec-Setup-{}.exe", r.version.replace(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-', "_")));
    let part = path.with_extension("part");
    let mut file = tokio::fs::File::create(&part).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut got = 0u64;
    let mut stream = resp.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.without_url().to_string())?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        got += chunk.len() as u64;
        tx(got, total);
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    let digest = hex::encode(hasher.finalize());
    if !digest.eq_ignore_ascii_case(&r.checksum_sha256) {
        let _ = std::fs::remove_file(&part);
        return Err("downloaded installer does not match the signed checksum; discarded".into());
    }
    std::fs::rename(&part, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Start the verified installer and let the caller exit. Windows only.
pub fn launch_installer(path: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new(path)
            .args(["/SILENT", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
            .spawn()
            .map_err(|e| format!("could not start installer: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(format!("installer saved to {} (automatic install is Windows-only)", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn release(sig: Option<String>) -> Release {
        Release {
            version: "1.2.3".into(),
            download_url: "https://psychosec.org/dl/Psychosec-Setup.exe".into(),
            checksum_sha256: "a".repeat(64),
            signature: sig,
            published_at: None,
            notes: None,
        }
    }

    #[test]
    fn signature_roundtrip_and_tamper() {
        let sk = SigningKey::from_bytes(&[9u8; 32]);
        let pk = B64.encode(sk.verifying_key().to_bytes());
        let mut r = release(None);
        r.signature = Some(B64.encode(sk.sign(signed_message(&r).as_bytes()).to_bytes()));
        assert!(verify_manifest(&r, &pk).is_ok());
        let mut t = r.clone();
        t.download_url = "https://evil.example/x.exe".into();
        assert!(verify_manifest(&t, &pk).is_err());
        let mut t = r.clone();
        t.checksum_sha256 = "b".repeat(64);
        assert!(verify_manifest(&t, &pk).is_err());
        assert!(verify_manifest(&release(None), &pk).is_err());
        let other = B64.encode(SigningKey::from_bytes(&[1u8; 32]).verifying_key().to_bytes());
        assert!(verify_manifest(&r, &other).is_err());
    }
    #[test]
    fn version_compare() {
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("garbage", "1.0.0"));
    }
    #[test]
    fn rejects_http_and_bad_checksum() {
        let mut r = release(None);
        r.download_url = "http://x/y.exe".into();
        assert!(check_url(&r).is_err());
        let mut r = release(None);
        r.checksum_sha256 = "zz".into();
        assert!(check_url(&r).is_err());
    }
}

#[cfg(test)]
mod cross_tests {
    use super::*;
    /// Verifies a manifest produced by scripts/release-sign.mjs (generated at test time if node is present).
    #[test]
    fn node_signed_manifest_verifies() {
        let dir = std::env::temp_dir().join(format!("psy-sign-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let run = |args: &[&str]| std::process::Command::new("node").args(args).current_dir(&dir).output();
        let Ok(k) = run(&[root.join("scripts/release-keygen.mjs").to_str().unwrap()]) else { return };
        if !k.status.success() { return; }
        let out = String::from_utf8(k.stdout).unwrap();
        let pubkey = out.lines().next().unwrap().trim_start_matches("PSYCHOSEC_UPDATE_PUBKEY=").to_string();
        std::fs::write(dir.join("a.exe"), b"installer-bytes").unwrap();
        let s = run(&[root.join("scripts/release-sign.mjs").to_str().unwrap(), "a.exe", "2.0.1", "https://psychosec.org/dl/x.exe"]).unwrap();
        assert!(s.status.success(), "{}", String::from_utf8_lossy(&s.stderr));
        let v: serde_json::Value = serde_json::from_slice(&s.stdout).unwrap();
        let r = Release {
            version: v["version"].as_str().unwrap().into(),
            download_url: v["download_url"].as_str().unwrap().into(),
            checksum_sha256: v["checksum_sha256"].as_str().unwrap().into(),
            signature: v["signature"].as_str().map(|s| s.into()),
            published_at: None,
            notes: None,
        };
        assert!(verify_manifest(&r, &pubkey).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }
}
