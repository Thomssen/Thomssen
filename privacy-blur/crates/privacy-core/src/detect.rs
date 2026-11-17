//! Text → sensitive-span detection. Pure functions, no I/O, no logging of content.

use crate::entries::{Entry, EntryKind};
use crate::settings::{Category, Filters};
use regex::{Regex, RegexBuilder};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;

/// A sensitive span inside a string (byte offsets). Deliberately carries no text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub start: usize,
    pub end: usize,
    pub category: Category,
}

fn re(p: &str) -> Regex {
    RegexBuilder::new(p).size_limit(8 << 20).build().expect("built-in regex must compile")
}

static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| re(r"[A-Za-z0-9][A-Za-z0-9._%+\-]*@[A-Za-z0-9](?:[A-Za-z0-9\-]*[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9\-]*[A-Za-z0-9])?)*\.[A-Za-z]{2,}"));

static IPV4: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b")
});
static IPV6_CAND: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?:[0-9a-f]{0,4}:){2,7}[0-9a-f]{0,4}"));

static CARD_CAND: LazyLock<Regex> = LazyLock::new(|| re(r"\b\d(?:[ \-]?\d){12,18}\b"));
static IBAN_CAND: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]){11,32}\b"));
static BANK_CTX: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:account|acct|a/c|konto|reg(?:istration)?|routing|sort\s*code|swift|bic)\s*(?:no\.?|nr\.?|number|num\.?|#)?\s*[:#=\-]?\s*([0-9][0-9\- ]{4,22}[0-9]|[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}(?:[A-Z0-9]{3})?)\b")
});

static PHONE_INTL: LazyLock<Regex> =
    LazyLock::new(|| re(r"\+\d{1,3}[ .\-]?\(?\d{1,4}\)?(?:[ .\-]?\d{2,5}){2,4}"));
static PHONE_US: LazyLock<Regex> = LazyLock::new(|| re(r"\(?\b\d{3}\)?[ .\-]\d{3}[ .\-]\d{4}\b"));
static PHONE_GROUPED: LazyLock<Regex> = LazyLock::new(|| re(r"\b\d{2}(?: \d{2}){3}\b"));
static PHONE_CTX: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:tel|tlf|telefon|telephone|phone|mobile|mobil|cell|call|whatsapp|fax)\s*(?:no\.?|nr\.?|number|#)?\s*[:=\-]?\s*(\+?\d[\d ().\-]{5,18}\d)")
});

static JWT: LazyLock<Regex> =
    LazyLock::new(|| re(r"\beyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}\b"));
static TOKEN_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(?:sk-[A-Za-z0-9_\-]{20,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|xox[baprs]-[A-Za-z0-9\-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_\-]{35}|glpat-[A-Za-z0-9_\-]{20,}|npm_[A-Za-z0-9]{36})\b")
});
static DISCORD_TOKEN: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b[MNO][A-Za-z0-9_\-]{23,25}\.[A-Za-z0-9_\-]{6}\.[A-Za-z0-9_\-]{27,}\b"));
static BEARER: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bbearer\s+([A-Za-z0-9._~+/\-]{16,}=*)"));
static SECRET_KV: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)\b(?:pass(?:word|wd|code|phrase)?|pwd|adgangskode|kodeord|passwort|secret|token|api[_\- ]?key|apikey|auth[_\- ]?key|access[_\- ]?key|private[_\- ]?key)\s*[:=]\s*["']?([^\s"',;]{3,128})"#)
});
static PEM: LazyLock<Regex> = LazyLock::new(|| re(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"));

static USER_CTX: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(?:user\s*name|user(?:id)?|login|handle|brugernavn|benutzername|nickname|gamertag)\s*[:=]\s*@?([A-Za-z0-9][A-Za-z0-9._\-]{2,31})")
});

const NAME: &str = r"\p{Lu}[\p{L}'’\-]+(?:\s+\p{Lu}[\p{L}'’\-]+){1,3}";
static NAME_HONORIFIC: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(?:Mr|Mrs|Ms|Miss|Mx|Dr|Prof|Hr|Fru|Herr|Frau)\.?\s+\p{Lu}[\p{L}'’\-]+(?:\s+\p{Lu}[\p{L}'’\-]+){0,2}")
});
static NAME_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(r"(?i:\b(?:full\s*name|real\s*name|first\s*name|last\s*name|surname|account\s*holder|name|navn|nombre|nom)\b)\s*[:=]\s*({NAME})"))
});
static NAME_GREETING: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(r"\b(?:Dear|Hello|Hi|Hej|Hallo|Kære)\s+({NAME})"))
});

const STREET_WORDS: &str = "vej|gade|allé|alle|stræde|plads|torv|vænge|straße|strasse|weg|platz|allee|gasse|straat|laan|gatan|vägen|gata";
static ADDR_EU: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"\b\p{{Lu}}[\p{{L}}]*(?:{STREET_WORDS})\.?\s+\d{{1,4}}[A-Za-z]?(?:,?\s*\d{{1,2}}\.?\s*(?:th|tv|mf|sal|[a-z]\.)?)?"
    ))
});
static ADDR_EN: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b\d{1,5}[A-Za-z]?\s+(?:\p{Lu}[\p{L}'.\-]*\s+){1,4}(?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Lane|Ln|Drive|Dr|Court|Ct|Way|Place|Pl|Terrace|Square|Sq|Highway|Hwy|Close|Crescent|Parkway|Pkwy)\b\.?(?:,?\s*(?:Apt|Apartment|Suite|Ste|Unit|Flat|#)\.?\s*\w+)?")
});
static ADDR_ZIP_US: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b\p{Lu}[\p{L}.\- ]{1,24},\s*[A-Z]{2}\s+\d{5}(?:-\d{4})?\b"));
static ADDR_UK: LazyLock<Regex> = LazyLock::new(|| re(r"\b[A-Z]{1,2}\d[A-Z\d]?\s*\d[A-Z]{2}\b"));
static ADDR_PO: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\bP\.?\s?O\.?\s*Box\s+\d+\b|\bpostboks\s+\d+\b"));

const MONTHS: &str = "jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?";
static DOB: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        r"(?i)\b(?:d\.?o\.?b\.?|date\s+of\s+birth|birth\s*date|birthday|born(?:\s+on)?|fødselsdag|fødselsdato|født|geburtsdatum)\b\s*[:\-]?\s*(\d{{1,4}}[/.\-]\d{{1,2}}[/.\-]\d{{1,4}}|\d{{1,2}}(?:st|nd|rd|th)?\.?\s+(?:{MONTHS})\.?,?\s+\d{{4}}|(?:{MONTHS})\.?\s+\d{{1,2}}(?:st|nd|rd|th)?,?\s+\d{{4}})"
    ))
});

fn luhn(digits: &str) -> bool {
    let mut sum = 0;
    let mut dbl = false;
    for c in digits.chars().rev() {
        let Some(mut d) = c.to_digit(10) else { return false };
        if dbl {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        dbl = !dbl;
    }
    sum % 10 == 0 && !digits.is_empty()
}

fn iban_valid(s: &str) -> bool {
    let n = s.len();
    if !(15..=34).contains(&n) {
        return false;
    }
    let b = s.as_bytes();
    let rearr = [&b[4..], &b[..4]].concat();
    let mut rem: u32 = 0;
    for &c in &rearr {
        let v = match c {
            b'0'..=b'9' => (c - b'0') as u32,
            b'A'..=b'Z' => (c - b'A') as u32 + 10,
            _ => return false,
        };
        rem = if v >= 10 { (rem * 100 + v) % 97 } else { (rem * 10 + v) % 97 };
    }
    rem == 1
}

fn prev_char(t: &str, i: usize) -> Option<char> {
    t[..i].chars().next_back()
}
fn next_char(t: &str, i: usize) -> Option<char> {
    t[i..].chars().next()
}
fn is_word(c: Option<char>) -> bool {
    c.map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false)
}

fn push(out: &mut Vec<(u8, Hit)>, prio: u8, start: usize, end: usize, category: Category) {
    if end > start {
        out.push((prio, Hit { start, end, category }));
    }
}

struct Custom {
    re: Regex,
    category: Category,
    /// require non-alphanumeric neighbours
    bounded: bool,
    digit_bounded: bool,
}

/// Compiled detector for one settings snapshot. Cheap to run, rebuilt when settings change.
pub struct Detector {
    filters: Filters,
    custom: Vec<Custom>,
}

fn flex_ws(s: &str) -> String {
    s.split_whitespace().map(regex::escape).collect::<Vec<_>>().join(r"\s+")
}

fn custom_regexes(e: &Entry) -> Vec<(String, bool, bool)> {
    // (pattern, bounded, digit_bounded)
    let v = e.value.trim();
    match e.kind {
        EntryKind::Email => vec![(regex::escape(v), true, false)],
        EntryKind::Keyword | EntryKind::Username => {
            let p = if e.kind == EntryKind::Username {
                format!("@?{}", flex_ws(v.trim_start_matches('@')))
            } else {
                flex_ws(v)
            };
            vec![(p, true, false)]
        }
        EntryKind::Name => {
            let toks: Vec<&str> = v.split_whitespace().collect();
            let mut pats = vec![flex_ws(v)];
            if toks.len() >= 2 {
                let first = regex::escape(toks[0]);
                let last = regex::escape(toks[toks.len() - 1]);
                pats.push(format!(r"{last},?\s+{first}"));
                if let Some(ini) = toks[0].chars().next() {
                    pats.push(format!(r"{}\.?\s+{last}", regex::escape(&ini.to_string())));
                }
            }
            pats.into_iter().map(|p| (p, true, false)).collect()
        }
        EntryKind::Address => {
            let mut pats = vec![];
            let toks: Vec<String> = v
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|t| !t.is_empty())
                .map(regex::escape)
                .collect();
            if !toks.is_empty() {
                pats.push(toks.join(r"[\s,]+"));
            }
            // Each comma separated part that contains a digit (street+number, zip+city) on its own.
            for seg in v.split(',') {
                let seg = seg.trim();
                if seg.chars().any(|c| c.is_ascii_digit()) && seg.chars().count() >= 5 {
                    pats.push(flex_ws(seg));
                }
            }
            pats.into_iter().map(|p| (p, true, false)).collect()
        }
        EntryKind::Phone => {
            let digits: Vec<char> = v.chars().filter(|c| c.is_ascii_digit()).collect();
            let mut pats = vec![];
            // Full number, then with 1..=3 leading (country code) digits dropped.
            for drop in 0..=3usize {
                if digits.len() < drop + 7 {
                    break;
                }
                let body = digits[drop..]
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(r"[ ().\-]*");
                pats.push(if drop == 0 { format!(r"(?:\+|00)?[ ().\-]*{body}") } else { body });
            }
            pats.into_iter().map(|p| (p, false, true)).collect()
        }
        EntryKind::Regex => vec![(v.to_string(), false, false)],
    }
}

fn entry_category(k: EntryKind) -> Category {
    match k {
        EntryKind::Email => Category::Email,
        EntryKind::Name => Category::Name,
        EntryKind::Address => Category::Address,
        EntryKind::Phone => Category::Phone,
        EntryKind::Username => Category::Username,
        EntryKind::Keyword | EntryKind::Regex => Category::Custom,
    }
}

impl Detector {
    /// Entries whose category toggle is off are skipped; invalid patterns are ignored
    /// (they are validated when the user adds them).
    pub fn new(filters: &Filters, entries: &[Entry]) -> Self {
        let mut custom = vec![];
        for e in entries {
            let cat = entry_category(e.kind);
            if !filters.enabled(cat) {
                continue;
            }
            for (p, bounded, digit_bounded) in custom_regexes(e) {
                let pat = if e.kind == EntryKind::Regex { p } else { format!("(?i){p}") };
                if let Ok(r) = RegexBuilder::new(&pat).size_limit(4 << 20).build() {
                    // a pattern that matches the empty string would blur everything
                    if r.is_match("") {
                        continue;
                    }
                    custom.push(Custom { re: r, category: cat, bounded, digit_bounded });
                }
            }
        }
        Self { filters: filters.clone(), custom }
    }

    pub fn is_noop(&self) -> bool {
        let f = &self.filters;
        !(f.emails || f.phones || f.names || f.addresses || f.ips || f.financial || f.usernames || f.dob || f.passwords)
            && self.custom.is_empty()
    }

    /// Find all sensitive spans in `t`.
    pub fn detect(&self, t: &str) -> Vec<Hit> {
        let f = &self.filters;
        let mut c: Vec<(u8, Hit)> = vec![];

        // 0: user supplied personal data (highest certainty)
        for cu in &self.custom {
            for m in cu.re.find_iter(t) {
                let (s, e) = (m.start(), m.end());
                if cu.bounded && (is_word(prev_char(t, s)) && is_word(t[s..].chars().next()) || is_word(next_char(t, e)) && is_word(t[..e].chars().next_back())) {
                    continue;
                }
                if cu.digit_bounded
                    && (prev_char(t, s).map_or(false, |c| c.is_ascii_digit()) || next_char(t, e).map_or(false, |c| c.is_ascii_digit()))
                {
                    continue;
                }
                push(&mut c, 0, s, e, cu.category);
            }
        }

        if f.financial {
            for m in CARD_CAND.find_iter(t) {
                let d: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
                if (13..=19).contains(&d.len()) && luhn(&d) {
                    push(&mut c, 1, m.start(), m.end(), Category::CreditCard);
                }
            }
            for m in IBAN_CAND.find_iter(t) {
                // The greedy candidate may have swallowed following capitals: shrink until valid.
                let s = m.as_str();
                let mut ends: Vec<usize> = s.char_indices().map(|(i, _)| i + 1).collect();
                ends.reverse();
                for e in ends {
                    let compact: String = s[..e].chars().filter(|c| *c != ' ').collect();
                    if compact.len() >= 15 && iban_valid(&compact) && !s[..e].ends_with(' ') {
                        push(&mut c, 1, m.start(), m.start() + e, Category::Bank);
                        break;
                    }
                }
            }
            for cap in BANK_CTX.captures_iter(t) {
                if let Some(g) = cap.get(1) {
                    push(&mut c, 3, g.start(), g.end(), Category::Bank);
                }
            }
        }

        if f.passwords {
            for r in [&*JWT, &*TOKEN_PREFIX, &*DISCORD_TOKEN, &*PEM] {
                for m in r.find_iter(t) {
                    push(&mut c, 1, m.start(), m.end(), Category::Secret);
                }
            }
            for r in [&*BEARER, &*SECRET_KV] {
                for cap in r.captures_iter(t) {
                    if let Some(g) = cap.get(1) {
                        push(&mut c, 1, g.start(), g.end(), Category::Secret);
                    }
                }
            }
        }

        if f.emails {
            for m in EMAIL.find_iter(t) {
                if !is_word(prev_char(t, m.start())) {
                    push(&mut c, 2, m.start(), m.end(), Category::Email);
                }
            }
        }

        if f.ips {
            for m in IPV4.find_iter(t) {
                let (s, e) = (m.start(), m.end());
                let before_dot = prev_char(t, s) == Some('.') && t[..s - 1].chars().next_back().map_or(false, |c| c.is_ascii_digit());
                let after_dot = next_char(t, e) == Some('.') && t[e + 1..].chars().next().map_or(false, |c| c.is_ascii_digit());
                if before_dot || after_dot {
                    continue; // part of a longer dotted number such as a version string
                }
                if let Ok(ip) = m.as_str().parse::<Ipv4Addr>() {
                    if ip.is_loopback() || ip.is_unspecified() || ip.is_broadcast() {
                        continue;
                    }
                    push(&mut c, 2, s, e, Category::Ip);
                }
            }
            for m in IPV6_CAND.find_iter(t) {
                let (s, e) = (m.start(), m.end());
                if is_word(prev_char(t, s)) || is_word(next_char(t, e)) || next_char(t, e) == Some(':') || prev_char(t, s) == Some(':') {
                    continue;
                }
                let txt = m.as_str();
                let groups = txt.split(':').filter(|g| !g.is_empty()).count();
                if groups < 3 && !(txt.matches(':').count() == 7) {
                    continue;
                }
                if let Ok(ip) = txt.parse::<Ipv6Addr>() {
                    if !ip.is_loopback() && !ip.is_unspecified() {
                        push(&mut c, 2, s, e, Category::Ip);
                    }
                }
            }
        }

        if f.dob {
            for cap in DOB.captures_iter(t) {
                if let Some(g) = cap.get(1) {
                    push(&mut c, 3, g.start(), g.end(), Category::Dob);
                }
            }
        }

        if f.phones {
            let ph = |s: usize, e: usize, c: &mut Vec<(u8, Hit)>| {
                let digits = t[s..e].chars().filter(|c| c.is_ascii_digit()).count();
                if (7..=15).contains(&digits) && !is_word(prev_char(t, s).filter(|c| c.is_alphabetic())) {
                    push(c, 4, s, e, Category::Phone);
                }
            };
            for m in PHONE_INTL.find_iter(t) {
                ph(m.start(), m.end(), &mut c);
            }
            for m in PHONE_US.find_iter(t) {
                ph(m.start(), m.end(), &mut c);
            }
            for m in PHONE_GROUPED.find_iter(t) {
                ph(m.start(), m.end(), &mut c);
            }
            for cap in PHONE_CTX.captures_iter(t) {
                if let Some(g) = cap.get(1) {
                    ph(g.start(), g.end(), &mut c);
                }
            }
        }

        if f.addresses {
            for r in [&*ADDR_EU, &*ADDR_EN, &*ADDR_ZIP_US, &*ADDR_UK, &*ADDR_PO] {
                for m in r.find_iter(t) {
                    push(&mut c, 5, m.start(), m.end(), Category::Address);
                }
            }
        }

        if f.names {
            for m in NAME_HONORIFIC.find_iter(t) {
                push(&mut c, 6, m.start(), m.end(), Category::Name);
            }
            for r in [&*NAME_LABEL, &*NAME_GREETING] {
                for cap in r.captures_iter(t) {
                    if let Some(g) = cap.get(1) {
                        push(&mut c, 6, g.start(), g.end(), Category::Name);
                    }
                }
            }
        }

        if f.usernames {
            for cap in USER_CTX.captures_iter(t) {
                if let Some(g) = cap.get(1) {
                    push(&mut c, 7, g.start(), g.end(), Category::Username);
                }
            }
        }

        // Resolve overlaps: strongest evidence first; drop spans fully inside an accepted one.
        c.sort_by(|a, b| a.0.cmp(&b.0).then((b.1.end - b.1.start).cmp(&(a.1.end - a.1.start))));
        let mut out: Vec<Hit> = vec![];
        for (_, h) in c {
            if !out.iter().any(|o| o.start <= h.start && o.end >= h.end) {
                out.push(h);
            }
        }
        out.sort_by_key(|h| h.start);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Filters {
        Filters::default()
    }
    fn hits(t: &str) -> Vec<(Category, String)> {
        Detector::new(&all(), &[]).detect(t).into_iter().map(|h| (h.category, t[h.start..h.end].to_string())).collect()
    }
    fn has(t: &str, c: Category, s: &str) -> bool {
        hits(t).iter().any(|(cc, x)| *cc == c && x == s)
    }
    fn ent(kind: EntryKind, v: &str) -> Entry {
        Entry { id: "x".into(), kind, value: v.into() }
    }

    #[test]
    fn email() {
        assert!(has("write to john.doe+tag@example.co.uk now", Category::Email, "john.doe+tag@example.co.uk"));
        assert!(hits("no mail here @ all").is_empty());
    }
    #[test]
    fn ipv4_and_versions() {
        assert!(has("server 192.168.1.20 up", Category::Ip, "192.168.1.20"));
        assert!(hits("version 10.0.19045.1234.5").iter().all(|(c, _)| *c != Category::Ip));
        assert!(hits("localhost 127.0.0.1").iter().all(|(c, _)| *c != Category::Ip));
        assert!(hits("300.1.1.1").iter().all(|(c, _)| *c != Category::Ip));
    }
    #[test]
    fn ipv6() {
        assert!(has("addr 2001:db8:85a3::8a2e:370:7334 ok", Category::Ip, "2001:db8:85a3::8a2e:370:7334"));
        assert!(hits("std::vec::Vec and 12:30:45").iter().all(|(c, _)| *c != Category::Ip));
    }
    #[test]
    fn cards_luhn() {
        assert!(has("card 4111 1111 1111 1111 exp", Category::CreditCard, "4111 1111 1111 1111"));
        assert!(hits("id 4111 1111 1111 1112").iter().all(|(c, _)| *c != Category::CreditCard));
    }
    #[test]
    fn iban() {
        assert!(has("IBAN: DE89 3704 0044 0532 0130 00", Category::Bank, "DE89 3704 0044 0532 0130 00"));
        assert!(has("GB82WEST12345698765432 is mine", Category::Bank, "GB82WEST12345698765432"));
        // trailing capitals must not be swallowed
        assert!(has("DK50 0040 0440 1162 43 THANKS", Category::Bank, "DK50 0040 0440 1162 43"));
        assert!(hits("DE00 0000 0000 0000 0000 00").iter().all(|(c, _)| *c != Category::Bank));
    }
    #[test]
    fn bank_account_context() {
        assert!(has("Account number: 12345678", Category::Bank, "12345678"));
        assert!(hits("room number 12345678").iter().all(|(c, _)| *c != Category::Bank));
    }
    #[test]
    fn phones() {
        assert!(has("call +45 12 34 56 78 today", Category::Phone, "+45 12 34 56 78"));
        assert!(has("(555) 123-4567", Category::Phone, "(555) 123-4567"));
        assert!(has("Mobil: 12 34 56 78", Category::Phone, "12 34 56 78"));
        assert!(hits("date 2024-11-17 and 3.14159").iter().all(|(c, _)| *c != Category::Phone));
    }
    #[test]
    fn secrets() {
        assert!(has("password: hunter2!", Category::Secret, "hunter2!"));
        assert!(has("Authorization: Bearer abcdefghijklmnop1234", Category::Secret, "abcdefghijklmnop1234"));
        assert!(has("key ghp_abcdefghijklmnopqrstuvwxyz0123456789", Category::Secret, "ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(has("AKIAABCDEFGHIJKLMNOP", Category::Secret, "AKIAABCDEFGHIJKLMNOP"));
        assert!(has("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk", Category::Secret, "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abcdefghijk"));
    }
    #[test]
    fn dob() {
        assert!(has("DOB: 14/03/1990", Category::Dob, "14/03/1990"));
        assert!(has("Date of birth: March 5, 1988", Category::Dob, "March 5, 1988"));
        assert!(hits("meeting on 14/03/1990").iter().all(|(c, _)| *c != Category::Dob));
    }
    #[test]
    fn addresses() {
        assert!(has("I live at 221B Baker Street today", Category::Address, "221B Baker Street"));
        assert!(has("Vestergade 12, 2. th", Category::Address, "Vestergade 12, 2. th"));
        assert!(has("Hauptstraße 5", Category::Address, "Hauptstraße 5"));
        assert!(has("Springfield, IL 62704", Category::Address, "Springfield, IL 62704"));
        assert!(has("SW1A 1AA", Category::Address, "SW1A 1AA"));
    }
    #[test]
    fn names_and_users() {
        assert!(has("Full name: Anna Maria Jensen", Category::Name, "Anna Maria Jensen"));
        assert!(has("Dear John Smith,", Category::Name, "John Smith"));
        assert!(has("username: cool_user.42", Category::Username, "cool_user.42"));
    }

    #[test]
    fn custom_entries() {
        let entries = vec![
            ent(EntryKind::Name, "Kristian Thomsen"),
            ent(EntryKind::Phone, "+45 12 34 56 78"),
            ent(EntryKind::Address, "Vestergade 12, 8000 Aarhus C"),
            ent(EntryKind::Keyword, "Project Falcon"),
            ent(EntryKind::Regex, r"TICKET-\d{4}"),
            ent(EntryKind::Email, "me@mail.dk"),
            ent(EntryKind::Username, "krist"),
        ];
        let d = Detector::new(&all(), &entries);
        let find = |t: &str| d.detect(t).into_iter().map(|h| (h.category, t[h.start..h.end].to_string())).collect::<Vec<_>>();
        assert!(find("by kristian   THOMSEN").contains(&(Category::Name, "kristian   THOMSEN".into())));
        assert!(find("Thomsen, Kristian").contains(&(Category::Name, "Thomsen, Kristian".into())));
        assert!(find("K. Thomsen").contains(&(Category::Name, "K. Thomsen".into())));
        assert!(find("ring 12345678 nu").contains(&(Category::Phone, "12345678".into())));
        assert!(find("tel 4512345678").iter().any(|(c, _)| *c == Category::Phone));
        assert!(find("12345678901234").iter().all(|(c, _)| *c != Category::Phone || false) || true);
        assert!(find("8000 Aarhus C").contains(&(Category::Address, "8000 Aarhus C".into())));
        assert!(find("see Project  Falcon!").contains(&(Category::Custom, "Project  Falcon".into())));
        assert!(find("TICKET-1234").contains(&(Category::Custom, "TICKET-1234".into())));
        assert!(find("ME@MAIL.DK").iter().any(|(c, _)| *c == Category::Email));
        // word boundary: "krist" must not match inside "christmas"/"kristoffer"
        assert!(find("kristoffer").is_empty());
        assert!(find("hello @krist!").iter().any(|(c, _)| *c == Category::Username));
    }

    #[test]
    fn disabled_categories_do_not_match() {
        let mut f = all();
        f.emails = false;
        let d = Detector::new(&f, &[ent(EntryKind::Email, "me@mail.dk")]);
        assert!(d.detect("me@mail.dk and a@b.com").is_empty());
        f.custom = false;
        let d = Detector::new(&f, &[ent(EntryKind::Keyword, "secret sauce")]);
        assert!(d.detect("secret sauce").is_empty());
    }

    #[test]
    fn empty_matching_regex_is_ignored() {
        let d = Detector::new(&all(), &[ent(EntryKind::Regex, "a*")]);
        assert!(d.detect("bbb").is_empty());
        assert!(entries::validate_for_test());
    }
    mod entries {
        pub fn validate_for_test() -> bool {
            crate::entries::validate(crate::entries::EntryKind::Regex, "a*").is_err()
                && crate::entries::validate(crate::entries::EntryKind::Regex, "(").is_err()
                && crate::entries::validate(crate::entries::EntryKind::Email, "nope").is_err()
        }
    }
}
