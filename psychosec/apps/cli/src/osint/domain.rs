use super::dns::{query, Rec};
use super::{client, err_str, tls, valid_hostname};
use crate::output::{Output, Status};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

pub fn normalize(input: &str) -> Result<String, String> {
    let d = input.trim().trim_start_matches("http://").trim_start_matches("https://");
    let d = d.split(['/', '?', '#']).next().unwrap_or("").trim_end_matches('.').to_ascii_lowercase();
    if valid_hostname(&d) {
        Ok(d)
    } else {
        Err(format!("'{input}' is not a valid domain name"))
    }
}

pub async fn scan(input: &str, timeout: u64) -> Result<Output, String> {
    let domain = normalize(input)?;
    let http = client(timeout)?;
    let mut o = Output::new("domain", &domain, &format!("Domain scan: {domain}"));

    let (a, aaaa, ns, mx, txt, caa, soa) = tokio::join!(
        query(&http, &domain, Rec::A),
        query(&http, &domain, Rec::Aaaa),
        query(&http, &domain, Rec::Ns),
        query(&http, &domain, Rec::Mx),
        query(&http, &domain, Rec::Txt),
        query(&http, &domain, Rec::Caa),
        query(&http, &domain, Rec::Soa),
    );
    if let Ok(l) = &a {
        if l.nxdomain {
            return Err(format!("{domain} does not exist (NXDOMAIN)"));
        }
    }
    o.source("https://cloudflare-dns.com/dns-query");
    o.heading("DNS");
    for (label, res) in [("A", a), ("AAAA", aaaa), ("NS", ns), ("MX", mx), ("TXT", txt), ("CAA", caa), ("SOA", soa)] {
        match res {
            Ok(l) if l.records.is_empty() => {
                o.kv(label, "none");
            }
            Ok(l) => {
                for r in l.records {
                    o.kv(label, r);
                }
            }
            Err(e) => {
                o.item(Status::Err, label, format!("lookup failed: {e}"));
            }
        }
    }

    // RDAP, TLS and CT run concurrently.
    let rdap_url = format!("https://rdap.org/domain/{domain}");
    let ct_url = format!("https://crt.sh/?q=%25.{domain}&output=json");
    let (rdap, ct, tlsr) = tokio::join!(
        async { http.get(&rdap_url).header("accept", "application/rdap+json").send().await },
        async { http.get(&ct_url).timeout(Duration::from_secs(timeout.max(25))).send().await },
        tls::inspect(&domain, 443, Duration::from_secs(timeout)),
    );

    o.heading("Registration (RDAP)");
    o.source(rdap_url);
    match rdap {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => rdap_domain(&mut o, &v),
            Err(e) => {
                o.item(Status::Err, "RDAP", format!("unreadable response: {e}"));
            }
        },
        Ok(r) => {
            o.item(Status::Warn, "RDAP", format!("no data (HTTP {}). The TLD may not publish RDAP.", r.status()));
        }
        Err(e) => {
            o.item(Status::Err, "RDAP", format!("lookup failed: {}", err_str(e)));
        }
    }

    o.heading("TLS certificate (port 443)");
    match tlsr {
        Ok(t) => {
            o.item(Status::Ok, "Handshake", format!("{} / {} (chain verified)", t.version, t.cipher));
            o.kv("Subject", t.leaf.subject).kv("Issuer", t.leaf.issuer);
            o.kv("Valid", format!("{} to {}", t.leaf.not_before, t.leaf.not_after));
            let st = if t.leaf.days_left < 0 { Status::Err } else if t.leaf.days_left < 21 { Status::Warn } else { Status::Ok };
            o.item(st, "Expiry", format!("{} days remaining", t.leaf.days_left));
            o.kv("Names (SAN)", t.leaf.sans.iter().take(12).cloned().collect::<Vec<_>>().join(", "));
            o.kv("Serial", t.leaf.serial);
        }
        Err(e) => {
            o.item(Status::Warn, "TLS", e);
        }
    }

    o.heading("Certificate transparency / subdomains (crt.sh)");
    o.source(format!("https://crt.sh/?q=%25.{domain}"));
    match ct {
        Ok(r) if r.status().is_success() => match r.json::<Vec<Value>>().await {
            Ok(rows) => {
                let mut subs = BTreeSet::new();
                for row in &rows {
                    if let Some(nv) = row.get("name_value").and_then(|v| v.as_str()) {
                        for n in nv.lines() {
                            let n = n.trim().trim_start_matches("*.").to_ascii_lowercase();
                            if n == domain || n.ends_with(&format!(".{domain}")) {
                                subs.insert(n);
                            }
                        }
                    }
                }
                o.kv("Logged certificates", rows.len().to_string());
                o.kv("Unique names", subs.len().to_string());
                for s in subs.iter().take(60) {
                    o.item(Status::Info, "", s.clone());
                }
                if subs.len() > 60 {
                    o.text(format!("... and {} more", subs.len() - 60));
                }
            }
            Err(e) => {
                o.item(Status::Err, "crt.sh", format!("unreadable response: {e}"));
            }
        },
        Ok(r) => {
            o.item(Status::Warn, "crt.sh", format!("service returned HTTP {}", r.status()));
        }
        Err(e) => {
            o.item(Status::Err, "crt.sh", format!("lookup failed: {}", err_str(e)));
        }
    }
    Ok(o)
}

fn rdap_domain(o: &mut Output, v: &Value) {
    if let Some(n) = v.get("ldhName").and_then(|x| x.as_str()) {
        o.kv("Domain", n);
    }
    if let Some(s) = v.get("status").and_then(|x| x.as_array()) {
        o.kv("Status", s.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", "));
    }
    if let Some(ev) = v.get("events").and_then(|x| x.as_array()) {
        for e in ev {
            if let (Some(a), Some(d)) = (e.get("eventAction").and_then(|x| x.as_str()), e.get("eventDate").and_then(|x| x.as_str())) {
                o.kv(a, d);
            }
        }
    }
    if let Some(ents) = v.get("entities").and_then(|x| x.as_array()) {
        for e in ents {
            let roles = e.get("roles").and_then(|r| r.as_array()).map(|r| r.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("/")).unwrap_or_default();
            if roles.contains("registrar") {
                if let Some(name) = vcard_fn(e) {
                    o.kv("Registrar", name);
                }
            }
        }
    }
    if let Some(ns) = v.get("nameservers").and_then(|x| x.as_array()) {
        o.kv("Nameservers", ns.iter().filter_map(|n| n.get("ldhName").and_then(|x| x.as_str())).collect::<Vec<_>>().join(", "));
    }
    if let Some(d) = v.get("secureDNS").and_then(|s| s.get("delegationSigned")).and_then(|x| x.as_bool()) {
        o.kv("DNSSEC", if d { "signed" } else { "unsigned" });
    }
}

pub fn vcard_fn(e: &Value) -> Option<String> {
    let arr = e.get("vcardArray")?.as_array()?.get(1)?.as_array()?;
    for p in arr {
        let p = p.as_array()?;
        if p.first()?.as_str()? == "fn" {
            return p.get(3)?.as_str().map(|s| s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes() {
        assert_eq!(normalize("https://Example.COM/path?x=1").unwrap(), "example.com");
        assert!(normalize("not a domain").is_err());
        assert!(normalize("10.0.0.1").is_err());
    }
    #[test]
    fn rdap_parse() {
        let v: Value = serde_json::json!({
            "ldhName":"EXAMPLE.COM","status":["active"],
            "events":[{"eventAction":"registration","eventDate":"1995-08-14T04:00:00Z"}],
            "entities":[{"roles":["registrar"],"vcardArray":["vcard",[["version",{},"text","4.0"],["fn",{},"text","Reg Inc"]]]}]
        });
        let mut o = Output::new("domain", "x", "x");
        rdap_domain(&mut o, &v);
        let t = o.plain_lines().join("\n");
        assert!(t.contains("Registrar: Reg Inc") && t.contains("registration: 1995"));
    }
}
