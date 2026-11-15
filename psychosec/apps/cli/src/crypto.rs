//! Field-level encryption for local case data (ChaCha20-Poly1305).
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::RngCore;

pub struct Crypto {
    cipher: ChaCha20Poly1305,
}

impl Crypto {
    pub fn from_key(key: [u8; 32]) -> Self {
        Self { cipher: ChaCha20Poly1305::new(Key::from_slice(&key)) }
    }

    /// Load the per-install key from the credential store, creating it on first run.
    pub fn load_or_create() -> Result<Self, String> {
        if let Some(k) = crate::creds::get(crate::creds::DB_KEY) {
            if let Ok(bytes) = B64.decode(k.trim()) {
                if let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) {
                    return Ok(Self::from_key(arr));
                }
            }
        }
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        crate::creds::set(crate::creds::DB_KEY, &B64.encode(key))
            .map_err(|e| format!("cannot store encryption key: {e}"))?;
        Ok(Self::from_key(key))
    }

    pub fn encrypt(&self, plain: &str) -> String {
        let mut nonce = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let ct = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plain.as_bytes())
            .expect("encryption failure");
        let mut buf = nonce.to_vec();
        buf.extend(ct);
        format!("v1:{}", B64.encode(buf))
    }

    pub fn decrypt(&self, s: &str) -> Result<String, String> {
        let b = s.strip_prefix("v1:").ok_or("unrecognised ciphertext")?;
        let raw = B64.decode(b).map_err(|_| "corrupt ciphertext")?;
        if raw.len() < 13 {
            return Err("corrupt ciphertext".into());
        }
        let (n, ct) = raw.split_at(12);
        let pt = self
            .cipher
            .decrypt(Nonce::from_slice(n), ct)
            .map_err(|_| "decryption failed (wrong key or tampered data)".to_string())?;
        String::from_utf8(pt).map_err(|_| "invalid utf-8".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_tamper() {
        let c = Crypto::from_key([7u8; 32]);
        let e = c.encrypt("secret note");
        assert_ne!(e, "secret note");
        assert_eq!(c.decrypt(&e).unwrap(), "secret note");
        let other = Crypto::from_key([8u8; 32]);
        assert!(other.decrypt(&e).is_err());
        assert!(c.decrypt("v1:AAAA").is_err());
    }
}
