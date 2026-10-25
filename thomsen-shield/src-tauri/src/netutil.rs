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

/// Guards every manual/UI "block this IP" entry point before it ever
/// reaches the firewall layer. Unlike the allowlist (which only narrows
/// what the *automatic* detection engine can touch), nothing previously
/// stopped a manual block command from being handed a reserved-local
/// address or an overly broad range - up to a full "any address" CIDR -
/// with no pushback at all. Returns the human-readable reason a target is
/// rejected, or `None` if it's fine to block.
pub fn reject_unsafe_block_target(entry: &str) -> Option<String> {
    let Some((network, prefix)) = parse_cidr(entry) else {
        return Some(format!("\"{entry}\" isn't a valid IP address or CIDR range."));
    };
    // /24 (256 addresses) is already a generous ceiling for a manual block -
    // wide enough for a genuinely abusive subnet, nowhere near wide enough
    // to take out a real chunk of the internet by mistake.
    const MIN_PREFIX: u8 = 24;
    if prefix < MIN_PREFIX {
        return Some(format!("That range covers too many addresses to block at once (minimum /{MIN_PREFIX})."));
    }
    if is_reserved_local(network) {
        return Some("That address is on your own network (loopback or private LAN) and can't be blocked.".to_string());
    }
    None
}

/// The always-protected ranges from the spec: the whole loopback block, the
/// three RFC1918 private ranges, and link-local (APIPA fallback, and how
/// mDNS/WS-Discovery neighbors like printers and other LAN devices commonly
/// show up). Independent of whatever the user's allowlist currently
/// contains, so it can never be edited away by mistake.
pub fn is_reserved_local(ip: Ipv4Addr) -> bool {
    const RANGES: &[&str] = &["127.0.0.0/8", "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "169.254.0.0/16"];
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
        assert!(is_reserved_local(Ipv4Addr::new(127, 0, 0, 53)), "other loopback addresses, not just 127.0.0.1 exactly");
        assert!(is_reserved_local(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_reserved_local(Ipv4Addr::new(172, 16, 0, 5)));
        assert!(is_reserved_local(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_reserved_local(Ipv4Addr::new(169, 254, 1, 1)), "link-local (APIPA/mDNS-neighbor range)");
        assert!(!is_reserved_local(Ipv4Addr::new(185, 34, 22, 10)));
        assert!(!is_reserved_local(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn rejects_unsafe_manual_block_targets() {
        assert!(reject_unsafe_block_target("185.34.22.10").is_none(), "a single public address must be allowed");
        assert!(reject_unsafe_block_target("185.34.22.0/24").is_none(), "a /24 is within the allowed ceiling");
        assert!(reject_unsafe_block_target("not-an-ip").is_some());
        assert!(reject_unsafe_block_target("0.0.0.0/0").is_some(), "must reject an all-addresses range");
        assert!(reject_unsafe_block_target("185.34.0.0/16").is_some(), "must reject a range wider than /24");
        assert!(reject_unsafe_block_target("192.168.1.1").is_some(), "must reject the user's own LAN");
        assert!(reject_unsafe_block_target("127.0.0.1").is_some(), "must reject loopback");
        assert!(reject_unsafe_block_target("169.254.1.1").is_some(), "must reject link-local");
    }
}
