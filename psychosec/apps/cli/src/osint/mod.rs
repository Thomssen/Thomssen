//! OSINT modules. Each returns real data gathered from public sources or the local
//! machine, or an accurate error. Nothing is simulated.
pub mod dns;
pub mod domain;
pub mod email;
pub mod file;
pub mod ip;
pub mod tls;
pub mod username;
pub mod web;

use std::time::Duration;

pub const MODULES: &[(&str, &str)] = &[
    ("username", "Public profile discovery across supported platforms"),
    ("domain", "DNS, RDAP, TLS and certificate transparency"),
    ("ip", "Network ownership, reverse DNS, approximate geolocation"),
    ("email", "Syntax, mail DNS and authorised breach intelligence"),
    ("web", "HTTP headers, security posture and technology detection"),
    ("file", "Local file hashing and metadata extraction"),
    ("case", "Encrypted local investigation case management"),
    ("report", "HTML / JSON / CSV report export"),
];

pub fn client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("Psychosec/{} (+https://psychosec.org)", crate::config::VERSION))
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(timeout_secs.min(8)))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| e.to_string())
}

pub fn err_str(e: reqwest::Error) -> String {
    let e = e.without_url();
    if e.is_timeout() {
        "request timed out".into()
    } else if e.is_connect() {
        "connection failed".into()
    } else {
        e.to_string()
    }
}

/// Strict hostname validation (RFC 1123 labels, optional IDN already punycoded).
pub fn valid_hostname(h: &str) -> bool {
    let h = h.trim_end_matches('.');
    if h.is_empty() || h.len() > 253 || !h.contains('.') {
        return false;
    }
    h.split('.').all(|l| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }) && !h.split('.').last().unwrap().chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostnames() {
        assert!(valid_hostname("example.com"));
        assert!(valid_hostname("a-b.example.co.uk."));
        assert!(!valid_hostname("localhost"));
        assert!(!valid_hostname("-bad.com"));
        assert!(!valid_hostname("exa mple.com"));
        assert!(!valid_hostname("1.2.3.4"));
        assert!(!valid_hostname(&format!("{}.com", "a".repeat(64))));
    }
}
