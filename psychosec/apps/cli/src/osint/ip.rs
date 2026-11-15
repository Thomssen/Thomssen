use super::dns::{ptr_name, query, Rec};
use super::{client, err_str};
use crate::output::{Output, Status};
use serde_json::Value;
use std::net::IpAddr;

pub fn is_public(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let o = v.octets();
            !(v.is_private() || v.is_loopback() || v.is_link_local() || v.is_broadcast() || v.is_unspecified()
                || v.is_multicast() || o[0] == 0 || (o[0] == 100 && (64..128).contains(&o[1])) || o[0] >= 240
                || (o[0] == 192 && o[1] == 0 && o[2] == 0) || (o[0] == 198 && (o[1] == 18 || o[1] == 19)))
        }
        IpAddr::V6(v) => {
            let s = v.segments();
            !(v.is_loopback() || v.is_unspecified() || v.is_multicast() || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80
                || (s[0] == 0x2001 && s[1] == 0x0db8))
        }
    }
}

pub async fn lookup(input: &str, timeout: u64) -> Result<Output, String> {
    let ip: IpAddr = input.trim().parse().map_err(|_| format!("'{input}' is not a valid IP address"))?;
    let mut o = Output::new("ip", &ip.to_string(), &format!("IP lookup: {ip}"));
    if !is_public(&ip) {
        o.item(Status::Warn, "Address", "private, loopback or reserved range: no public registration data exists");
        return Ok(o);
    }
    let http = client(timeout)?;
    let rdap_url = format!("https://rdap.org/ip/{ip}");
    let geo_url = format!("https://ipwho.is/{ip}");
    let ptr_q = ptr_name(ip);
    let (rdap, geo, ptr) = tokio::join!(
        async { http.get(&rdap_url).header("accept", "application/rdap+json").send().await },
        async { http.get(&geo_url).send().await },
        query(&http, &ptr_q, Rec::Ptr),
    );

    o.heading("Network registration (RDAP)");
    o.source(rdap_url);
    match rdap {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => {
                for (k, label) in [("name", "Network"), ("handle", "Handle"), ("country", "Country"), ("type", "Type")] {
                    if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
                        o.kv(label, s);
                    }
                }
                if let (Some(a), Some(b)) = (v.get("startAddress").and_then(|x| x.as_str()), v.get("endAddress").and_then(|x| x.as_str())) {
                    o.kv("Range", format!("{a} - {b}"));
                }
                if let Some(c) = v.get("cidr0_cidrs").and_then(|x| x.as_array()) {
                    let s: Vec<String> = c
                        .iter()
                        .filter_map(|c| Some(format!("{}/{}", c.get("v4prefix").or(c.get("v6prefix"))?.as_str()?, c.get("length")?.as_u64()?)))
                        .collect();
                    if !s.is_empty() {
                        o.kv("CIDR", s.join(", "));
                    }
                }
                if let Some(ents) = v.get("entities").and_then(|x| x.as_array()) {
                    for e in ents {
                        if let Some(n) = super::domain::vcard_fn(e) {
                            let roles = e.get("roles").and_then(|r| r.as_array()).map(|r| r.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("/")).unwrap_or_default();
                            o.kv(if roles.is_empty() { "Entity".to_string() } else { roles }, n);
                        }
                    }
                }
            }
            Err(e) => {
                o.item(Status::Err, "RDAP", format!("unreadable response: {e}"));
            }
        },
        Ok(r) => {
            o.item(Status::Warn, "RDAP", format!("no data (HTTP {})", r.status()));
        }
        Err(e) => {
            o.item(Status::Err, "RDAP", format!("lookup failed: {}", err_str(e)));
        }
    }

    o.heading("ASN and approximate location");
    o.source(geo_url);
    match geo {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) if v.get("success").and_then(|x| x.as_bool()) == Some(true) => {
                let s = |p: &str| v.pointer(p).and_then(|x| x.as_str()).map(|x| x.to_string());
                if let Some(asn) = v.pointer("/connection/asn").and_then(|x| x.as_u64()) {
                    o.kv("ASN", format!("AS{asn}"));
                }
                if let Some(x) = s("/connection/org") { o.kv("Organisation", x); }
                if let Some(x) = s("/connection/isp") { o.kv("ISP", x); }
                if let Some(x) = s("/connection/domain") { o.kv("Network domain", x); }
                let loc = [s("/city"), s("/region"), s("/country")].into_iter().flatten().collect::<Vec<_>>().join(", ");
                if !loc.is_empty() {
                    o.kv("Location (approx.)", loc);
                }
                o.text("Geolocation is statistical and approximate (often city- or region-level and frequently wrong for mobile, VPN and hosting ranges). It is not a physical address.");
            }
            Ok(v) => {
                o.item(Status::Warn, "Geolocation", v.get("message").and_then(|x| x.as_str()).unwrap_or("provider returned no data").to_string());
            }
            Err(e) => {
                o.item(Status::Err, "Geolocation", format!("unreadable response: {e}"));
            }
        },
        Ok(r) => {
            o.item(Status::Warn, "Geolocation", format!("provider returned HTTP {}", r.status()));
        }
        Err(e) => {
            o.item(Status::Err, "Geolocation", format!("lookup failed: {}", err_str(e)));
        }
    }

    o.heading("Reverse DNS");
    match ptr {
        Ok(l) if l.records.is_empty() => {
            o.kv("PTR", "none");
        }
        Ok(l) => {
            for r in l.records {
                o.kv("PTR", r);
            }
        }
        Err(e) => {
            o.item(Status::Err, "PTR", format!("lookup failed: {e}"));
        }
    }

    o.heading("Abuse reputation");
    match crate::config::env_key("ABUSEIPDB_API_KEY") {
        None => {
            o.item(Status::Dim, "AbuseIPDB", "not configured (set the ABUSEIPDB_API_KEY environment variable to enable)");
        }
        Some(key) => {
            o.source("https://api.abuseipdb.com/api/v2/check");
            let r = http
                .get("https://api.abuseipdb.com/api/v2/check")
                .query(&[("ipAddress", ip.to_string()), ("maxAgeInDays", "90".into())])
                .header("Key", key)
                .header("Accept", "application/json")
                .send()
                .await;
            match r {
                Ok(r) if r.status().is_success() => match r.json::<Value>().await {
                    Ok(v) => {
                        let d = &v["data"];
                        let score = d["abuseConfidenceScore"].as_u64().unwrap_or(0);
                        let st = if score >= 50 { Status::Err } else if score > 0 { Status::Warn } else { Status::Ok };
                        o.item(st, "Confidence score", format!("{score}/100 over the last 90 days"));
                        o.kv("Reports", d["totalReports"].to_string());
                        o.kv("Usage type", d["usageType"].as_str().unwrap_or("unknown"));
                    }
                    Err(e) => {
                        o.item(Status::Err, "AbuseIPDB", format!("unreadable response: {e}"));
                    }
                },
                Ok(r) => {
                    o.item(Status::Err, "AbuseIPDB", format!("HTTP {} (check your API key / quota)", r.status()));
                }
                Err(e) => {
                    o.item(Status::Err, "AbuseIPDB", format!("lookup failed: {}", err_str(e)));
                }
            }
        }
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_ranges() {
        for p in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(is_public(&p.parse().unwrap()), "{p}");
        }
        for p in ["10.0.0.1", "192.168.1.1", "127.0.0.1", "169.254.1.1", "100.64.0.1", "::1", "fe80::1", "fc00::1", "224.0.0.1", "2001:db8::1"] {
            assert!(!is_public(&p.parse().unwrap()), "{p}");
        }
    }
}
