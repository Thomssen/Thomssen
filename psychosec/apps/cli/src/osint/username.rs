//! Username discovery on public sites using each site's public API / profile
//! endpoint. Only unauthenticated, public lookups; no scraping behind logins.
use super::{client, err_str};
use crate::output::{Output, Status};
use futures_util::stream::{self, StreamExt};

#[derive(Clone, Copy)]
pub enum Check {
    /// 200 = exists, 404 = free.
    Status,
    /// 200 + body must NOT contain marker ("null" style APIs).
    BodyNotEquals(&'static str),
    /// 200 + body must contain marker.
    BodyContains(&'static str),
}

struct Site {
    name: &'static str,
    /// `{}` is replaced by the username.
    api: &'static str,
    profile: &'static str,
    check: Check,
    /// Permitted username pattern for the platform (conservative).
    max_len: usize,
}

static SITES: &[Site] = &[
    Site { name: "GitHub", api: "https://api.github.com/users/{}", profile: "https://github.com/{}", check: Check::Status, max_len: 39 },
    Site { name: "GitLab", api: "https://gitlab.com/api/v4/users?username={}", profile: "https://gitlab.com/{}", check: Check::BodyNotEquals("[]"), max_len: 255 },
    Site { name: "Codeberg", api: "https://codeberg.org/api/v1/users/{}", profile: "https://codeberg.org/{}", check: Check::Status, max_len: 40 },
    Site { name: "Hacker News", api: "https://hacker-news.firebaseio.com/v0/user/{}.json", profile: "https://news.ycombinator.com/user?id={}", check: Check::BodyNotEquals("null"), max_len: 15 },
    Site { name: "npm", api: "https://registry.npmjs.org/-/user/org.couchdb.user:{}", profile: "https://www.npmjs.com/~{}", check: Check::Status, max_len: 214 },
    Site { name: "Docker Hub", api: "https://hub.docker.com/v2/users/{}/", profile: "https://hub.docker.com/u/{}", check: Check::Status, max_len: 30 },
    Site { name: "Dev.to", api: "https://dev.to/api/users/by_username?url={}", profile: "https://dev.to/{}", check: Check::Status, max_len: 30 },
    Site { name: "Keybase", api: "https://keybase.io/_/api/1.0/user/lookup.json?usernames={}", profile: "https://keybase.io/{}", check: Check::BodyContains("\"them\":[{"), max_len: 16 },
    Site { name: "Chess.com", api: "https://api.chess.com/pub/player/{}", profile: "https://www.chess.com/member/{}", check: Check::Status, max_len: 25 },
    Site { name: "Lichess", api: "https://lichess.org/api/user/{}", profile: "https://lichess.org/@/{}", check: Check::Status, max_len: 20 },
    Site { name: "Mastodon.social", api: "https://mastodon.social/api/v1/accounts/lookup?acct={}", profile: "https://mastodon.social/@{}", check: Check::Status, max_len: 30 },
    Site { name: "Gravatar", api: "https://en.gravatar.com/{}.json", profile: "https://gravatar.com/{}", check: Check::Status, max_len: 64 },
    Site { name: "Replit", api: "https://replit.com/@{}", profile: "https://replit.com/@{}", check: Check::Status, max_len: 30 },
    Site { name: "PyPI", api: "https://pypi.org/user/{}/", profile: "https://pypi.org/user/{}/", check: Check::Status, max_len: 40 },
    Site { name: "Reddit", api: "https://www.reddit.com/user/{}/about.json", profile: "https://www.reddit.com/user/{}", check: Check::Status, max_len: 20 },
];

pub fn valid_username(u: &str) -> Result<(), String> {
    if u.is_empty() || u.len() > 64 {
        return Err("username must be 1-64 characters".into());
    }
    if !u.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err("username may only contain letters, digits, '.', '_' and '-'".into());
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Confirmed,
    Available,
    Potential(String),
    Skipped(String),
    Error(String),
}

pub fn judge(check: Check, status: u16, body: &str) -> Verdict {
    match (status, check) {
        (200, Check::Status) => Verdict::Confirmed,
        (200, Check::BodyNotEquals(m)) => if body.trim() == m { Verdict::Available } else { Verdict::Confirmed },
        (200, Check::BodyContains(m)) => if body.replace(' ', "").contains(m) { Verdict::Confirmed } else { Verdict::Available },
        (404, Check::Status) | (404, _) | (410, _) => Verdict::Available,
        (429, _) => Verdict::Error("rate limited by the site; try again later".into()),
        (401 | 403, _) => Verdict::Potential(format!("site blocked the automated check (HTTP {status}); verify manually")),
        (s, _) => Verdict::Error(format!("unexpected HTTP {s}")),
    }
}

async fn check_site(http: reqwest::Client, s: &'static Site, username: String) -> (&'static Site, Verdict) {
    if username.len() > s.max_len {
        return (s, Verdict::Skipped(format!("longer than the site's {}-char limit", s.max_len)));
    }
    let url = s.api.replace("{}", &username);
    let v = match http.get(&url).header("accept", "application/json, text/html;q=0.8").send().await {
        Ok(r) => {
            let status = r.status().as_u16();
            let body = if matches!(s.check, Check::Status) { String::new() } else { r.text().await.unwrap_or_default() };
            judge(s.check, status, &body)
        }
        Err(e) => Verdict::Error(err_str(e)),
    };
    (s, v)
}

pub async fn search(username: &str, timeout: u64) -> Result<Output, String> {
    let username: &str = username.trim().trim_start_matches('@');
    valid_username(username)?;
    let username = username.to_string();
    let username = username.as_str();
    let http = client(timeout)?;
    let mut o = Output::new("username", username, &format!("Username search: {username}"));
    let results: Vec<(&'static Site, Verdict)> = stream::iter(0..SITES.len())
        .map(|i| check_site(http.clone(), &SITES[i], username.to_string()))
        .buffer_unordered(6)
        .collect()
        .await;
    let mut results = results;
    results.sort_by_key(|(s, _)| s.name);
    let (mut found, mut avail, mut pot, mut err) = (0, 0, 0, 0);
    o.heading("Confirmed profiles");
    for (s, v) in &results {
        if *v == Verdict::Confirmed {
            found += 1;
            let url = s.profile.replace("{}", username);
            o.item(Status::Ok, s.name, url.clone());
            o.source(url);
        }
    }
    if found == 0 { o.text("none"); }
    o.heading("Potential matches (manual verification needed)");
    let mut any = false;
    for (s, v) in &results {
        if let Verdict::Potential(why) = v {
            pot += 1;
            any = true;
            o.item(Status::Warn, s.name, format!("{} - {}", s.profile.replace("{}", username), why));
        }
    }
    if !any { o.text("none"); }
    o.heading("Not found / username available");
    for (s, v) in &results {
        match v {
            Verdict::Available => { avail += 1; o.item(Status::Dim, s.name, "no profile (name appears available)"); }
            Verdict::Skipped(w) => { o.item(Status::Dim, s.name, format!("skipped: {w}")); }
            _ => {}
        }
    }
    let errs: Vec<_> = results.iter().filter_map(|(s, v)| if let Verdict::Error(e) = v { Some((s.name, e)) } else { None }).collect();
    if !errs.is_empty() {
        o.heading("Checks that failed");
        for (n, e) in errs { err += 1; o.item(Status::Err, n, e.clone()); }
    }
    o.heading("Summary");
    o.kv("Checked", format!("{} platforms", SITES.len()));
    o.kv("Confirmed", found.to_string()).kv("Potential", pot.to_string()).kv("Available", avail.to_string()).kv("Errors", err.to_string());
    o.text("A matching username shows an account exists, not that it belongs to a particular person. Confirm identity through independent evidence.");
    if err == SITES.len() {
        return Err("every platform check failed; check your network connection".into());
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn username_validation() {
        assert!(valid_username("alice_01").is_ok());
        for b in ["", "a b", "a/b", "a@b", &"x".repeat(65)] {
            assert!(valid_username(b).is_err(), "{b}");
        }
    }
    #[test]
    fn verdicts() {
        assert_eq!(judge(Check::Status, 200, ""), Verdict::Confirmed);
        assert_eq!(judge(Check::Status, 404, ""), Verdict::Available);
        assert_eq!(judge(Check::BodyNotEquals("null"), 200, "null\n"), Verdict::Available);
        assert_eq!(judge(Check::BodyNotEquals("[]"), 200, "[{\"id\":1}]"), Verdict::Confirmed);
        assert_eq!(judge(Check::BodyContains("\"them\":[{"), 200, "{\"them\": [{\"a\":1}]}"), Verdict::Confirmed);
        assert!(matches!(judge(Check::Status, 403, ""), Verdict::Potential(_)));
        assert!(matches!(judge(Check::Status, 429, ""), Verdict::Error(_)));
        assert!(matches!(judge(Check::Status, 500, ""), Verdict::Error(_)));
    }
}
