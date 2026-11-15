use super::{client, err_str, tls};
use crate::output::{Output, Status};
use reqwest::header::HeaderMap;
use std::time::Duration;

pub fn normalize(input: &str) -> Result<url::Url, String> {
    let s = input.trim();
    let s = if s.contains("://") { s.to_string() } else { format!("https://{s}") };
    let u = url::Url::parse(&s).map_err(|_| format!("'{input}' is not a valid URL"))?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err("only http and https URLs are supported".into());
    }
    if u.host_str().is_none() {
        return Err("URL has no host".into());
    }
    Ok(u)
}

const SEC_HEADERS: &[(&str, &str)] = &[
    ("strict-transport-security", "Forces HTTPS (HSTS)"),
    ("content-security-policy", "Restricts script/content sources"),
    ("x-content-type-options", "Disables MIME sniffing"),
    ("x-frame-options", "Clickjacking protection"),
    ("referrer-policy", "Controls referrer leakage"),
    ("permissions-policy", "Restricts browser features"),
    ("cross-origin-opener-policy", "Cross-origin isolation"),
];

pub fn analyze_headers(h: &HeaderMap, https: bool) -> Vec<(Status, String, String)> {
    let mut v = vec![];
    for (name, why) in SEC_HEADERS {
        if *name == "strict-transport-security" && !https {
            continue;
        }
        match h.get(*name).and_then(|x| x.to_str().ok()) {
            Some(val) => {
                let mut st = Status::Ok;
                let mut note = val.to_string();
                if *name == "content-security-policy" && (val.contains("'unsafe-inline'") || val.contains("'unsafe-eval'")) {
                    st = Status::Warn;
                    note = format!("{note}  (allows unsafe-inline/eval)");
                }
                if *name == "strict-transport-security" {
                    let age = val.split(';').find_map(|p| p.trim().strip_prefix("max-age=").and_then(|n| n.parse::<u64>().ok())).unwrap_or(0);
                    if age < 15_552_000 {
                        st = Status::Warn;
                        note = format!("{note}  (max-age under 180 days)");
                    }
                }
                v.push((st, name.to_string(), note));
            }
            None => v.push((Status::Warn, name.to_string(), format!("missing - {why}"))),
        }
    }
    v
}

pub fn detect_tech(h: &HeaderMap, body: &str) -> Vec<String> {
    let mut t = std::collections::BTreeSet::new();
    let hv = |n: &str| h.get(n).and_then(|x| x.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let server = hv("server");
    for (needle, name) in [("nginx", "nginx"), ("apache", "Apache"), ("cloudflare", "Cloudflare"), ("microsoft-iis", "Microsoft IIS"), ("litespeed", "LiteSpeed"), ("caddy", "Caddy"), ("vercel", "Vercel"), ("gws", "Google Web Server"), ("amazons3", "Amazon S3"), ("netlify", "Netlify")] {
        if server.contains(needle) {
            t.insert(name.to_string());
        }
    }
    let xp = hv("x-powered-by");
    for (needle, name) in [("php", "PHP"), ("express", "Express"), ("asp.net", "ASP.NET"), ("next.js", "Next.js")] {
        if xp.contains(needle) {
            t.insert(name.to_string());
        }
    }
    if h.contains_key("cf-ray") { t.insert("Cloudflare".into()); }
    if h.contains_key("x-vercel-id") { t.insert("Vercel".into()); }
    if h.contains_key("x-amz-cf-id") { t.insert("Amazon CloudFront".into()); }
    if h.contains_key("x-github-request-id") { t.insert("GitHub Pages".into()); }
    let b = body.to_ascii_lowercase();
    for (needle, name) in [
        ("wp-content/", "WordPress"), ("__next_data__", "Next.js"), ("/_next/static", "Next.js"), ("data-reactroot", "React"),
        ("ng-version", "Angular"), ("__nuxt", "Nuxt"), ("cdn.shopify.com", "Shopify"), ("drupal-settings-json", "Drupal"),
        ("content=\"joomla", "Joomla"), ("squarespace", "Squarespace"), ("wix.com", "Wix"), ("jquery", "jQuery"), ("bootstrap", "Bootstrap"),
        ("googletagmanager.com", "Google Tag Manager"), ("google-analytics.com", "Google Analytics"),
    ] {
        if b.contains(needle) {
            t.insert(name.to_string());
        }
    }
    if let Some(i) = b.find("name=\"generator\"") {
        let s = &body[i.saturating_sub(120)..(i + 160).min(body.len())];
        if let Some(c) = s.to_ascii_lowercase().find("content=\"") {
            let rest = &s[c + 9..];
            if let Some(e) = rest.find('"') {
                t.insert(format!("generator: {}", &rest[..e]));
            }
        }
    }
    t.into_iter().collect()
}

pub fn cookie_issues(h: &HeaderMap) -> Vec<String> {
    let mut v = vec![];
    for c in h.get_all("set-cookie") {
        if let Ok(s) = c.to_str() {
            let l = s.to_ascii_lowercase();
            let name = s.split('=').next().unwrap_or("?").to_string();
            let mut miss = vec![];
            if !l.contains("secure") { miss.push("Secure"); }
            if !l.contains("httponly") { miss.push("HttpOnly"); }
            if !l.contains("samesite") { miss.push("SameSite"); }
            if !miss.is_empty() {
                v.push(format!("{name}: missing {}", miss.join(", ")));
            }
        }
    }
    v
}

pub async fn inspect(input: &str, timeout: u64) -> Result<Output, String> {
    let url = normalize(input)?;
    let http = client(timeout)?;
    let mut o = Output::new("web", url.as_str(), &format!("Web inspect: {}", url.host_str().unwrap_or("")));
    o.text("Single passive GET request. Only inspect systems you own or are authorised to assess.");
    o.source(url.as_str());
    let resp = http.get(url.clone()).send().await.map_err(|e| format!("request failed: {}", err_str(e)))?;
    let status = resp.status();
    let final_url = resp.url().clone();
    let version = format!("{:?}", resp.version());
    let headers = resp.headers().clone();
    let remote = resp.remote_addr().map(|a| a.ip().to_string());
    let body = {
        use futures_util::StreamExt;
        let mut s = resp.bytes_stream();
        let mut buf: Vec<u8> = vec![];
        while let Some(Ok(c)) = s.next().await {
            buf.extend_from_slice(&c);
            if buf.len() > 1_000_000 {
                break;
            }
        }
        String::from_utf8_lossy(&buf).into_owned()
    };
    o.heading("Response");
    o.item(if status.is_success() { Status::Ok } else { Status::Warn }, "Status", status.to_string());
    o.kv("Protocol", version);
    if final_url != url {
        o.kv("Final URL", final_url.as_str());
    }
    if let Some(ip) = remote {
        o.kv("Server IP", ip);
    }
    for k in ["server", "content-type", "x-powered-by", "via", "last-modified"] {
        if let Some(v) = headers.get(k).and_then(|x| x.to_str().ok()) {
            o.kv(k, v);
        }
    }
    let https = final_url.scheme() == "https";
    o.heading("Security headers");
    let mut missing = 0;
    for (st, k, v) in analyze_headers(&headers, https) {
        if st != Status::Ok { missing += 1; }
        o.item(st, k, v);
    }
    o.kv("Summary", format!("{missing} header issue(s)"));
    let ci = cookie_issues(&headers);
    if !ci.is_empty() {
        o.heading("Cookies");
        for c in ci {
            o.item(Status::Warn, "cookie", c);
        }
    }
    o.heading("Technology (passive signatures)");
    let tech = detect_tech(&headers, &body);
    if tech.is_empty() {
        o.text("No known technology signatures detected.");
    } else {
        for t in tech {
            o.item(Status::Info, "", t);
        }
    }
    if https {
        o.heading("TLS");
        match tls::inspect(final_url.host_str().unwrap_or(""), final_url.port().unwrap_or(443), Duration::from_secs(timeout)).await {
            Ok(t) => {
                o.item(Status::Ok, "Protocol", format!("{} / {}", t.version, t.cipher));
                if let Some(a) = t.alpn { o.kv("ALPN", a); }
                o.kv("Issuer", t.leaf.issuer);
                o.item(if t.leaf.days_left < 21 { Status::Warn } else { Status::Ok }, "Certificate", format!("expires {} ({} days)", t.leaf.not_after, t.leaf.days_left));
            }
            Err(e) => { o.item(Status::Warn, "TLS", e); }
        }
    } else {
        o.item(Status::Warn, "Transport", "site is served over plain HTTP");
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn hm(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(HeaderName::from_bytes(k.as_bytes()).unwrap(), HeaderValue::from_str(v).unwrap());
        }
        h
    }
    #[test]
    fn url_normalisation() {
        assert_eq!(normalize("example.com/a").unwrap().as_str(), "https://example.com/a");
        assert!(normalize("ftp://x.com").is_err());
        assert!(normalize("http://").is_err());
    }
    #[test]
    fn header_analysis() {
        let r = analyze_headers(&hm(&[("strict-transport-security", "max-age=63072000"), ("content-security-policy", "default-src 'self' 'unsafe-inline'")]), true);
        let get = |n: &str| r.iter().find(|x| x.1 == n).unwrap().0;
        assert_eq!(get("strict-transport-security"), Status::Ok);
        assert_eq!(get("content-security-policy"), Status::Warn);
        assert_eq!(get("x-frame-options"), Status::Warn);
        assert!(analyze_headers(&hm(&[]), false).iter().all(|x| x.1 != "strict-transport-security"));
    }
    #[test]
    fn tech_and_cookies() {
        let t = detect_tech(&hm(&[("server", "nginx/1.25"), ("x-vercel-id", "x")]), "<script src=\"/_next/static/a.js\">");
        assert!(t.contains(&"nginx".to_string()) && t.contains(&"Next.js".to_string()) && t.contains(&"Vercel".to_string()));
        let c = cookie_issues(&hm(&[("set-cookie", "sid=1; Path=/"), ("set-cookie", "ok=1; Secure; HttpOnly; SameSite=Lax")]));
        assert_eq!(c.len(), 1);
    }
}
