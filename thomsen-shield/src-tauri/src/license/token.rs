//! Verifies the signed activation token the license API issues - see
//! `license/client.rs` for how it's fetched and `license/store.rs` for how
//! it's cached locally. The client only ever *verifies* with this embedded
//! public key; nothing that can *mint* a valid token ships in this binary,
//! which is the whole point of asymmetric signing here - see the module
//! doc on `license` for the fuller picture.

use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// The public half of the keypair generated for `api.thomssen.dev` via
/// `npm run generate-keypair` in `thomsen-shield-license-api/`. The
/// private half lives only in that Vercel project's environment
/// variables - see that project's README ("Signing keypair"). Embedding
/// a *public* key here is safe: it can verify a signature, never create
/// one.
///
/// Rotating keys later: generate a fresh pair, update
/// `LICENSE_SIGNING_PRIVATE_KEY_B64` / `LICENSE_SIGNING_PUBLIC_KEY_B64` in
/// Vercel, and paste the new public key's bytes in here in the same
/// commit - an old client build won't verify tokens from a rotated key
/// until it ships with the matching bytes.
///
/// Rotated twice before anything ever went live: the first keypair's
/// private half was pasted into a chat response, and its replacement's
/// private half was lost by both the user and this session (deliberately -
/// this session never keeps a copy once a secret's been handed off) before
/// it reached Vercel. Neither had any issued tokens to invalidate. This is
/// the third keypair's public half.
const LICENSE_SIGNING_PUBLIC_KEY: [u8; 32] = [
    161, 218, 88, 108, 228, 106, 74, 75, 163, 131, 39, 161, 123, 163, 98, 167, 108, 20, 144, 71, 174, 227, 211, 70, 129, 172, 192, 209, 91, 125, 78, 115,
];

/// camelCase to match the exact JSON the backend's `lib/signing.ts`
/// produces - this is what gets parsed out of `payload_json` below once
/// the signature over those bytes checks out, so the field casing has to
/// agree with the signer, not just be internally consistent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPayload {
    pub license_key: String,
    pub device_id: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// The wire/cache format - camelCase to match `SignedToken` in the
/// backend's `lib/signing.ts` exactly, since this struct is deserialized
/// directly from that API's JSON response as well as round-tripped
/// through the local DPAPI cache. `payload_json` is kept as the exact
/// string the signature was computed over - re-serializing the parsed
/// struct before verifying would risk a byte-for-byte mismatch (field
/// order, spacing) even when the data is logically identical, so the
/// signature is always checked against these exact bytes, never a
/// re-encoded copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedToken {
    pub payload_json: String,
    pub signature_b64: String,
}

impl SignedToken {
    /// `Ok(payload)` only if the signature is valid *and* the payload
    /// parses. Expiry is the caller's concern (`license::mod` compares
    /// `expires_at` against now for the offline grace period), since a
    /// technically-valid-but-expired token is still worth reading, e.g. to
    /// show "license expired" rather than "invalid license".
    pub fn verify(&self, public_key: &VerifyingKey) -> AppResult<TokenPayload> {
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.signature_b64)
            .map_err(|_| AppError::new("The saved license activation is corrupted."))?;
        let signature = Signature::from_slice(&sig_bytes).map_err(|_| AppError::new("The saved license activation is corrupted."))?;

        public_key
            .verify(self.payload_json.as_bytes(), &signature)
            .map_err(|_| AppError::new("The saved license activation failed verification."))?;

        serde_json::from_str(&self.payload_json).map_err(|_| AppError::new("The saved license activation is corrupted."))
    }
}

pub fn license_signing_public_key() -> VerifyingKey {
    VerifyingKey::from_bytes(&LICENSE_SIGNING_PUBLIC_KEY).expect("embedded license signing public key is a valid Ed25519 key")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sign(signing_key: &SigningKey, payload: &TokenPayload) -> SignedToken {
        let payload_json = serde_json::to_string(payload).unwrap();
        let signature = signing_key.sign(payload_json.as_bytes());
        SignedToken {
            payload_json,
            signature_b64: base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
        }
    }

    #[test]
    fn a_correctly_signed_token_verifies() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let payload = TokenPayload {
            license_key: "TSHIELD-AAAA-BBBB-CCCC".to_string(),
            device_id: "abc123".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(14),
        };
        let token = sign(&signing_key, &payload);
        let verified = token.verify(&verifying_key).expect("should verify");
        assert_eq!(verified.license_key, payload.license_key);
        assert_eq!(verified.device_id, payload.device_id);
    }

    #[test]
    fn a_token_signed_by_the_wrong_key_is_rejected() {
        let real_key = SigningKey::from_bytes(&[7u8; 32]);
        let attacker_key = SigningKey::from_bytes(&[9u8; 32]);
        let payload = TokenPayload {
            license_key: "TSHIELD-AAAA-BBBB-CCCC".to_string(),
            device_id: "abc123".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(14),
        };
        let token = sign(&attacker_key, &payload);
        assert!(token.verify(&real_key.verifying_key()).is_err());
    }

    #[test]
    fn a_tampered_payload_is_rejected() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let payload = TokenPayload {
            license_key: "TSHIELD-AAAA-BBBB-CCCC".to_string(),
            device_id: "abc123".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(14),
        };
        let mut token = sign(&signing_key, &payload);
        token.payload_json = token.payload_json.replace("abc123", "someone-elses-device");
        assert!(token.verify(&signing_key.verifying_key()).is_err());
    }
}
