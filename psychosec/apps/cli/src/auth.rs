//! Browser-based Discord login using a PKCE-style one-time verifier.
//! The CLI never sees Discord credentials or tokens; it only receives an application
//! session token from our backend after the backend completed the OAuth2 exchange.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64U, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub struct Pending {
    pub login_id: String,
    pub verifier: String,
    pub challenge: String,
}

fn rand_b64u(n: usize) -> String {
    let mut b = vec![0u8; n];
    rand::rngs::OsRng.fill_bytes(&mut b);
    B64U.encode(b)
}

pub fn challenge_for(verifier: &str) -> String {
    B64U.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn begin() -> Pending {
    let verifier = rand_b64u(48);
    Pending { login_id: rand_b64u(24), challenge: challenge_for(&verifier), verifier }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pkce_challenge_is_sha256_b64u() {
        // RFC 7636 appendix B test vector
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let p = begin();
        assert_eq!(p.challenge, challenge_for(&p.verifier));
        assert!(p.verifier.len() >= 43);
    }
}
