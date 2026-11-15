use super::dns::{query, Rec};
use super::{client, err_str, valid_hostname};
use crate::output::{Output, Status};
use serde_json::Value;

/// Practical RFC 5321/5322 syntax check (no quoted local parts / comments).
pub fn split_email(e: &str) -> Result<(String, String), String> {
    let e = e.trim();
    if e.len() > 254 {
        return Err("address exceeds 254 characters".into());
    }
    let (local, domain) = e.rsplit_once('@').ok_or("missing '@'")?;
    if local.is_empty() || local.len() > 64 {
        return Err("local part must be 1-64 characters".into());
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return Err("local part has misplaced dots".into());
    }
    if !local.chars().all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c)) {
        return Err("local part contains unsupported characters".into());
    }
    let domain = domain.to_ascii_lowercase();
    if !valid_hostname(&domain) {
        return Err("domain part is not a valid hostname".into());
    }
    Ok((local.to_string(), domain))
}

pub async fn check(input: &str, timeout: u64) -> Result<Output, String> {
    let (local, domain) = split_email(input).map_err(|e| format!("invalid email address: {e}"))?;
    let addr = format!("{local}@{domain}");
    let http = client(timeout)?;
    let mut o = Output::new("email", &addr, &format!("Email check: {addr}"));
    o.heading("Syntax");
    o.item(Status::Ok, "Format", "valid");
    o.kv("Domain", domain.clone());

    let dmarc_name = format!("_dmarc.{domain}");
    let mta_name = format!("_mta-sts.{domain}");
    let (mx, a, spf, dmarc, mta) = tokio::join!(
        query(&http, &domain, Rec::Mx),
        query(&http, &domain, Rec::A),
        query(&http, &domain, Rec::Txt),
        query(&http, &dmarc_name, Rec::Txt),
        query(&http, &mta_name, Rec::Txt),
    );
    o.source("https://cloudflare-dns.com/dns-query");
    o.heading("Mail DNS");
    match mx {
        Ok(l) if l.nxdomain => {
            o.item(Status::Err, "Domain", "does not exist (NXDOMAIN): this address cannot receive mail");
        }
        Ok(l) if l.records.is_empty() => {
            let has_a = a.as_ref().map(|x| !x.records.is_empty()).unwrap_or(false);
            o.item(if has_a { Status::Warn } else { Status::Err }, "MX", if has_a { "no MX records; mail would fall back to the A record" } else { "no MX or A records: cannot receive mail" });
        }
        Ok(l) => {
            let mut recs = l.records;
            recs.sort_by_key(|r| r.split_whitespace().next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(u32::MAX));
            for r in recs {
                o.item(Status::Ok, "MX", r);
            }
        }
        Err(e) => {
            o.item(Status::Err, "MX", format!("lookup failed: {e}"));
        }
    }
    let find = |r: Result<super::dns::Lookup, String>, prefix: &str| -> Result<Option<String>, String> {
        r.map(|l| l.records.into_iter().find(|t| t.to_ascii_lowercase().starts_with(prefix)))
    };
    match find(spf, "v=spf1") {
        Ok(Some(s)) => { o.item(Status::Ok, "SPF", s); }
        Ok(None) => { o.item(Status::Warn, "SPF", "no SPF record"); }
        Err(e) => { o.item(Status::Err, "SPF", format!("lookup failed: {e}")); }
    }
    match find(dmarc, "v=dmarc1") {
        Ok(Some(s)) => { o.item(Status::Ok, "DMARC", s); }
        Ok(None) => { o.item(Status::Warn, "DMARC", "no DMARC record"); }
        Err(e) => { o.item(Status::Err, "DMARC", format!("lookup failed: {e}")); }
    }
    match find(mta, "v=stsv1") {
        Ok(Some(s)) => { o.item(Status::Ok, "MTA-STS", s); }
        Ok(None) => { o.item(Status::Dim, "MTA-STS", "not published"); }
        Err(_) => {}
    }
    o.text("A valid format and mail DNS do not prove that this mailbox exists. No connection to the mail server is made.");

    o.heading("Breach intelligence (Have I Been Pwned)");
    match crate::config::env_key("HIBP_API_KEY") {
        None => {
            o.item(Status::Dim, "HIBP", "not configured (set the HIBP_API_KEY environment variable; keys are sold by haveibeenpwned.com)");
        }
        Some(key) => {
            let url = format!("https://haveibeenpwned.com/api/v3/breachedaccount/{}", urlencode(&addr));
            o.source("https://haveibeenpwned.com/api/v3/breachedaccount/");
            let r = http.get(&url).query(&[("truncateResponse", "false")]).header("hibp-api-key", key).send().await;
            match r {
                Ok(r) if r.status().as_u16() == 404 => {
                    o.item(Status::Ok, "Breaches", "address not found in any known breach");
                }
                Ok(r) if r.status().is_success() => match r.json::<Vec<Value>>().await {
                    Ok(b) => {
                        o.item(Status::Warn, "Breaches", format!("found in {} breach(es)", b.len()));
                        for x in b.iter().take(25) {
                            o.item(
                                Status::Info,
                                x["Name"].as_str().unwrap_or("?"),
                                format!("{} - {}", x["BreachDate"].as_str().unwrap_or("?"), x["DataClasses"].as_array().map(|d| d.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default()),
                            );
                        }
                    }
                    Err(e) => { o.item(Status::Err, "HIBP", format!("unreadable response: {e}")); }
                },
                Ok(r) if r.status().as_u16() == 401 => { o.item(Status::Err, "HIBP", "API key rejected"); }
                Ok(r) if r.status().as_u16() == 429 => { o.item(Status::Err, "HIBP", "rate limited, retry in a moment"); }
                Ok(r) => { o.item(Status::Err, "HIBP", format!("HTTP {}", r.status())); }
                Err(e) => { o.item(Status::Err, "HIBP", format!("lookup failed: {}", err_str(e))); }
            }
        }
    }
    Ok(o)
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn syntax() {
        assert!(split_email("user.name+tag@Example.com").is_ok());
        for bad in ["plain", "@x.com", "a@", "a..b@x.com", ".a@x.com", "a b@x.com", "a@localhost", "a@-x.com", &format!("{}@x.com", "a".repeat(65))] {
            assert!(split_email(bad).is_err(), "{bad}");
        }
        assert_eq!(split_email("A@Ex.COM").unwrap().1, "ex.com");
    }
    #[test]
    fn encoding() {
        assert_eq!(urlencode("a+b@x.com"), "a%2Bb%40x.com");
    }
}
