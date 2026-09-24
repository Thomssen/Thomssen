//! Pure IP/CIDR helpers - no OS calls, fully unit-testable. Shared by the
//! allowlist matcher and the detection engine so "is this address
//! protected?" is answered exactly the same way everywhere.

use std::net::Ipv4Addr;

/// Parses a single IPv4 address or a `a.b.c.d/nn` CIDR range into
/// `(network, prefix_len)`. A bare address is treated as a /32.
pub fn parse_cidr(entry: &str) -> Option<(Ipv4Addr, u8)> {
    let entry = entry.trim();
    if let Some((addr_part, prefix_part)) = entry.split_once('/') {
        let addr: Ipv4Addr = addr_part.trim().parse().ok()?;
        let prefix: u8 = prefix_part.trim().parse().ok()?;
        if prefix > 32 {
            return None;
        }
        Some((addr, prefix))
    } else {
        let addr: Ipv4Addr = entry.parse().ok()?;
        Some((addr, 32))
    }
}

/// True if `ip` falls inside the `entry` address or CIDR range. Invalid
/// entries never match anything, rather than erroring - a malformed saved
/// entry should not crash detection, just be inert.
pub fn ip_matches(ip: Ipv4Addr, entry: &str) -> bool {
    let Some((network, prefix)) = parse_cidr(entry) else {
        return false;
    };
    if prefix == 0 {
        return true;
    }
    let mask = u32::MAX.checked_shl(32 - prefix as u32).unwrap_or(0);
    u32::from(ip) & mask == u32::from(network) & mask
}

/// The always-protected ranges from the spec: loopback and the three
/// RFC1918 private ranges. Independent of whatever the user's allowlist
/// currently contains, so it can never be edited away by mistake.
pub fn is_reserved_local(ip: Ipv4Addr) -> bool {
    const RANGES: &[&str] = &["127.0.0.1", "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"];
    RANGES.iter().any(|r| ip_matches(ip, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_address_as_slash_32() {
        assert_eq!(parse_cidr("192.168.1.5"), Some((Ipv4Addr::new(192, 168, 1, 5), 32)));
    }

    #[test]
    fn parses_cidr_range() {
        assert_eq!(parse_cidr("10.0.0.0/8"), Some((Ipv4Addr::new(10, 0, 0, 0), 8)));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_cidr("not-an-ip"), None);
        assert_eq!(parse_cidr("10.0.0.0/99"), None);
        assert_eq!(parse_cidr(""), None);
    }

    #[test]
    fn matches_exact_address() {
        assert!(ip_matches(Ipv4Addr::new(127, 0, 0, 1), "127.0.0.1"));
        assert!(!ip_matches(Ipv4Addr::new(127, 0, 0, 2), "127.0.0.1"));
    }

    #[test]
    fn matches_cidr_range() {
        assert!(ip_matches(Ipv4Addr::new(10, 42, 6, 9), "10.0.0.0/8"));
        assert!(!ip_matches(Ipv4Addr::new(11, 0, 0, 1), "10.0.0.0/8"));
        assert!(ip_matches(Ipv4Addr::new(192, 168, 50, 12), "192.168.0.0/16"));
        assert!(ip_matches(Ipv4Addr::new(172, 20, 0, 1), "172.16.0.0/12"));
        assert!(!ip_matches(Ipv4Addr::new(172, 32, 0, 1), "172.16.0.0/12"));
    }

    #[test]
    fn reserved_local_covers_the_spec_ranges() {
        assert!(is_reserved_local(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_reserved_local(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_reserved_local(Ipv4Addr::new(172, 16, 0, 5)));
        assert!(is_reserved_local(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(!is_reserved_local(Ipv4Addr::new(185, 34, 22, 10)));
        assert!(!is_reserved_local(Ipv4Addr::new(8, 8, 8, 8)));
    }
}
