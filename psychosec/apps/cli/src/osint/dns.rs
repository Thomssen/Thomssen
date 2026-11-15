//! DNS over HTTPS (Cloudflare JSON API). Used so lookups behave identically on every
//! Windows network configuration. Queried names are sent to Cloudflare (documented
//! in the privacy policy).
use serde::Deserialize;

const DOH: &str = "https://cloudflare-dns.com/dns-query";

#[derive(Deserialize)]
struct Resp {
    #[serde(rename = "Status")]
    status: u32,
    #[serde(rename = "Answer", default)]
    answer: Vec<Ans>,
}
#[derive(Deserialize)]
struct Ans {
    #[serde(rename = "type")]
    typ: u32,
    data: String,
}

pub enum Rec {
    A,
    Aaaa,
    Ns,
    Soa,
    Ptr,
    Mx,
    Txt,
    Caa,
}
impl Rec {
    fn code(&self) -> u32 {
        match self {
            Rec::A => 1,
            Rec::Ns => 2,
            Rec::Soa => 6,
            Rec::Ptr => 12,
            Rec::Mx => 15,
            Rec::Txt => 16,
            Rec::Aaaa => 28,
            Rec::Caa => 257,
        }
    }
}

/// Returns the answer data for the requested type. `Ok(vec![])` means NOERROR with no
/// records or NXDOMAIN; `nx` reports NXDOMAIN separately.
pub struct Lookup {
    pub records: Vec<String>,
    pub nxdomain: bool,
}

pub async fn query(http: &reqwest::Client, name: &str, rec: Rec) -> Result<Lookup, String> {
    let resp = http
        .get(DOH)
        .query(&[("name", name), ("type", &rec.code().to_string())])
        .header("accept", "application/dns-json")
        .send()
        .await
        .map_err(super::err_str)?;
    if !resp.status().is_success() {
        return Err(format!("DNS resolver returned HTTP {}", resp.status()));
    }
    let r: Resp = resp.json().await.map_err(|e| format!("bad DNS response: {e}"))?;
    if r.status != 0 && r.status != 3 {
        return Err(format!("DNS resolver error (rcode {})", r.status));
    }
    let want = rec.code();
    let records = r
        .answer
        .into_iter()
        .filter(|a| a.typ == want)
        .map(|a| if want == 16 { clean_txt(&a.data) } else { a.data.trim_end_matches('.').to_string() })
        .collect();
    Ok(Lookup { records, nxdomain: r.status == 3 })
}

/// DoH returns TXT strings quoted, possibly split in several quoted chunks.
pub fn clean_txt(s: &str) -> String {
    let mut out = String::new();
    let mut in_q = false;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            out.push(c);
            esc = false;
        } else if c == '\\' && in_q {
            esc = true;
        } else if c == '"' {
            in_q = !in_q;
        } else if in_q {
            out.push(c);
        }
    }
    if out.is_empty() { s.to_string() } else { out }
}

pub fn ptr_name(ip: std::net::IpAddr) -> String {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            format!("{}.{}.{}.{}.in-addr.arpa", o[3], o[2], o[1], o[0])
        }
        std::net::IpAddr::V6(v6) => {
            let hex: String = v6.octets().iter().map(|b| format!("{:02x}", b)).collect();
            let nibbles: Vec<String> = hex.chars().rev().map(|c| c.to_string()).collect();
            format!("{}.ip6.arpa", nibbles.join("."))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn txt_cleaning() {
        assert_eq!(clean_txt("\"v=spf1 -all\""), "v=spf1 -all");
        assert_eq!(clean_txt("\"abc\" \"def\""), "abcdef");
        assert_eq!(clean_txt("\"a\\\"b\""), "a\"b");
    }
    #[test]
    fn ptr_names() {
        assert_eq!(ptr_name("1.2.3.4".parse().unwrap()), "4.3.2.1.in-addr.arpa");
        let v6 = ptr_name("2001:db8::1".parse().unwrap());
        assert!(v6.starts_with("1.0.0.0.") && v6.ends_with("8.b.d.0.1.0.0.2.ip6.arpa"));
    }
}
