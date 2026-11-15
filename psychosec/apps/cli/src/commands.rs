//! Modular command parser, completion and help.

#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    Html,
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help(Option<String>),
    Modules,
    Status,
    Account,
    Settings(Option<(String, String)>),
    Clear,
    Login,
    Logout,
    Exit,
    Update(bool),
    UsernameSearch(String),
    DomainScan(String),
    IpLookup(String),
    EmailCheck(String),
    WebInspect(String),
    FileMetadata(String),
    CaseCreate(String),
    CaseOpen(i64),
    CaseList,
    CaseFindings,
    CaseAdd(Option<String>),
    CaseDelete(i64),
    CaseRename(String),
    CaseNote(String),
    CaseVerify(i64, bool),
    CaseTarget(String),
    CaseClose,
    ReportExport(Format, Option<String>),
}

pub struct Spec {
    pub name: &'static str,
    pub usage: &'static str,
    pub desc: &'static str,
    pub group: &'static str,
}

pub const SPECS: &[Spec] = &[
    Spec { group: "General", name: "help", usage: "help [topic]", desc: "Show help" },
    Spec { group: "General", name: "modules", usage: "modules", desc: "List loaded modules" },
    Spec { group: "General", name: "status", usage: "status", desc: "Session, version and case status" },
    Spec { group: "General", name: "account", usage: "account", desc: "Show the authenticated operator" },
    Spec { group: "General", name: "settings", usage: "settings [set <key> <value>]", desc: "View or change settings" },
    Spec { group: "General", name: "update", usage: "update [install]", desc: "Check for / install a verified update" },
    Spec { group: "General", name: "clear", usage: "clear", desc: "Clear the screen" },
    Spec { group: "General", name: "login", usage: "login", desc: "Sign in with Discord" },
    Spec { group: "General", name: "logout", usage: "logout", desc: "End this session" },
    Spec { group: "General", name: "exit", usage: "exit", desc: "Quit" },
    Spec { group: "Investigate", name: "username search", usage: "username search <username>", desc: "Look for a username on public sites" },
    Spec { group: "Investigate", name: "domain scan", usage: "domain scan <domain>", desc: "DNS, RDAP, TLS, certificate transparency" },
    Spec { group: "Investigate", name: "ip lookup", usage: "ip lookup <ip>", desc: "ASN, network registration, rDNS, reputation" },
    Spec { group: "Investigate", name: "email check", usage: "email check <email>", desc: "Syntax, mail DNS, breach intelligence" },
    Spec { group: "Investigate", name: "web inspect", usage: "web inspect <url>", desc: "Headers, TLS, technology (single passive GET)" },
    Spec { group: "Investigate", name: "file metadata", usage: "file metadata <path>", desc: "Hash, type, EXIF/PDF/Office metadata" },
    Spec { group: "Cases", name: "case create", usage: "case create <name>", desc: "Create and open a case" },
    Spec { group: "Cases", name: "case open", usage: "case open <id>", desc: "Open a case" },
    Spec { group: "Cases", name: "case list", usage: "case list", desc: "List cases" },
    Spec { group: "Cases", name: "case findings", usage: "case findings", desc: "List findings in the active case" },
    Spec { group: "Cases", name: "case add", usage: "case add [note]", desc: "Save the last result as a finding" },
    Spec { group: "Cases", name: "case delete", usage: "case delete <id>", desc: "Delete a case (confirmation required)" },
    Spec { group: "Cases", name: "case rename", usage: "case rename <name>", desc: "Rename the active case" },
    Spec { group: "Cases", name: "case note", usage: "case note <text>", desc: "Add a note to the active case" },
    Spec { group: "Cases", name: "case target", usage: "case target <value>", desc: "Add an investigation target" },
    Spec { group: "Cases", name: "case verify", usage: "case verify <finding-id> [off]", desc: "Mark a finding verified/unverified" },
    Spec { group: "Cases", name: "case close", usage: "case close", desc: "Close the active case" },
    Spec { group: "Reports", name: "report export", usage: "report export --format html|json|csv [--out path]", desc: "Export the active case" },
];

pub const COMPLETIONS: &[&str] = &[
    "help", "modules", "status", "account", "settings", "settings set ", "update", "update install", "clear",
    "login", "logout", "exit", "username search ", "domain scan ", "ip lookup ", "email check ", "web inspect ",
    "file metadata ", "case create ", "case open ", "case list", "case findings", "case add", "case delete ",
    "case rename ", "case note ", "case target ", "case verify ", "case close", "report export --format html",
    "report export --format json", "report export --format csv",
];

pub fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has = false;
    for c in input.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => {
                quote = Some(c);
                has = true;
            }
            (None, c) if c.is_whitespace() => {
                if has || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

fn rest(args: &[String], from: usize) -> String {
    args[from..].join(" ")
}

fn need(args: &[String], from: usize, usage: &str) -> Result<String, String> {
    let r = rest(args, from);
    if r.trim().is_empty() {
        Err(format!("missing argument. usage: {usage}"))
    } else {
        Ok(r)
    }
}

fn id(s: &str) -> Result<i64, String> {
    s.trim_start_matches('#').parse::<i64>().map_err(|_| format!("'{s}' is not a valid id"))
}

pub fn parse(input: &str) -> Result<Command, String> {
    let a = tokenize(input)?;
    if a.is_empty() {
        return Err("empty command".into());
    }
    let c0 = a[0].to_ascii_lowercase();
    let c1 = a.get(1).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    let c0 = c0.trim_start_matches('/').to_string();
    match (c0.as_str(), c1.as_str()) {
        ("help", _) | ("?", _) => Ok(Command::Help(a.get(1).cloned())),
        ("modules", _) => Ok(Command::Modules),
        ("status", _) => Ok(Command::Status),
        ("account", _) => Ok(Command::Account),
        ("settings", "") => Ok(Command::Settings(None)),
        ("settings", "set") => {
            if a.len() != 4 {
                return Err("usage: settings set <key> <value>".into());
            }
            Ok(Command::Settings(Some((a[2].clone(), a[3].clone()))))
        }
        ("settings", _) => Err("usage: settings [set <key> <value>]".into()),
        ("update", "") => Ok(Command::Update(false)),
        ("update", "install") => Ok(Command::Update(true)),
        ("clear", _) | ("cls", _) => Ok(Command::Clear),
        ("login", _) => Ok(Command::Login),
        ("logout", _) => Ok(Command::Logout),
        ("exit", _) | ("quit", _) => Ok(Command::Exit),
        ("username", "search") => Ok(Command::UsernameSearch(need(&a, 2, "username search <username>")?)),
        ("domain", "scan") => Ok(Command::DomainScan(need(&a, 2, "domain scan <domain>")?)),
        ("ip", "lookup") => Ok(Command::IpLookup(need(&a, 2, "ip lookup <ip>")?)),
        ("email", "check") => Ok(Command::EmailCheck(need(&a, 2, "email check <email>")?)),
        ("web", "inspect") => Ok(Command::WebInspect(need(&a, 2, "web inspect <url>")?)),
        ("file", "metadata") => Ok(Command::FileMetadata(need(&a, 2, "file metadata <path>")?)),
        ("case", "create") => Ok(Command::CaseCreate(need(&a, 2, "case create <name>")?)),
        ("case", "open") => Ok(Command::CaseOpen(id(&need(&a, 2, "case open <id>")?)?)),
        ("case", "list") => Ok(Command::CaseList),
        ("case", "findings") => Ok(Command::CaseFindings),
        ("case", "add") => Ok(Command::CaseAdd(if a.len() > 2 { Some(rest(&a, 2)) } else { None })),
        ("case", "delete") => Ok(Command::CaseDelete(id(&need(&a, 2, "case delete <id>")?)?)),
        ("case", "rename") => Ok(Command::CaseRename(need(&a, 2, "case rename <name>")?)),
        ("case", "note") => Ok(Command::CaseNote(need(&a, 2, "case note <text>")?)),
        ("case", "target") => Ok(Command::CaseTarget(need(&a, 2, "case target <value>")?)),
        ("case", "verify") => {
            let fid = id(&need(&a, 2, "case verify <finding-id> [off]")?.split_whitespace().next().unwrap_or("").to_string())?;
            let off = a.get(3).map(|s| s.eq_ignore_ascii_case("off")).unwrap_or(false);
            Ok(Command::CaseVerify(fid, !off))
        }
        ("case", "close") => Ok(Command::CaseClose),
        ("case", "") => Err("usage: case create|open|list|findings|add|delete|rename|note|target|verify|close".into()),
        ("report", "export") => {
            let mut fmt = None;
            let mut out = None;
            let mut i = 2;
            while i < a.len() {
                match a[i].as_str() {
                    "--format" | "-f" => {
                        let v = a.get(i + 1).ok_or("--format needs a value (html, json, csv)")?;
                        fmt = Some(match v.to_ascii_lowercase().as_str() {
                            "html" => Format::Html,
                            "json" => Format::Json,
                            "csv" => Format::Csv,
                            o => return Err(format!("unknown format '{o}' (html, json, csv)")),
                        });
                        i += 2;
                    }
                    "--out" | "-o" => {
                        out = Some(a.get(i + 1).ok_or("--out needs a path")?.clone());
                        i += 2;
                    }
                    o => return Err(format!("unexpected argument '{o}'")),
                }
            }
            Ok(Command::ReportExport(fmt.ok_or("usage: report export --format html|json|csv")?, out))
        }
        ("report", _) => Err("usage: report export --format html|json|csv".into()),
        (other, _) => Err(format!("unknown command '{other}'. Type 'help'.")),
    }
}

/// Number of leading tokens that make up a recognised command name, for highlighting.
pub fn command_prefix_len(input: &str) -> Option<usize> {
    let lower = input.to_ascii_lowercase();
    let mut best = None;
    for s in SPECS {
        if lower == s.name || lower.starts_with(&format!("{} ", s.name)) {
            if best.map_or(true, |b: usize| s.name.len() > b) {
                best = Some(s.name.len());
            }
        }
    }
    best
}

pub fn complete(input: &str) -> Vec<&'static str> {
    let l = input.to_ascii_lowercase();
    COMPLETIONS.iter().copied().filter(|c| c.starts_with(&l) && *c != l).collect()
}

/// Usage hint for the (possibly partial) input.
pub fn hint(input: &str) -> Option<String> {
    let l = input.trim_start().to_ascii_lowercase();
    if l.is_empty() {
        return None;
    }
    let exact: Vec<&Spec> = SPECS
        .iter()
        .filter(|s| l == s.name || l.starts_with(&format!("{} ", s.name)))
        .collect();
    if let Some(s) = exact.iter().max_by_key(|s| s.name.len()) {
        return Some(format!("{}  -  {}", s.usage, s.desc));
    }
    let m: Vec<&Spec> = SPECS.iter().filter(|s| s.name.starts_with(&l)).collect();
    match m.len() {
        0 => None,
        1 => Some(format!("{}  -  {}", m[0].usage, m[0].desc)),
        _ => Some(m.iter().take(6).map(|s| s.name).collect::<Vec<_>>().join("   ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_investigation_commands() {
        assert_eq!(parse("username search alice").unwrap(), Command::UsernameSearch("alice".into()));
        assert_eq!(parse("domain scan example.com").unwrap(), Command::DomainScan("example.com".into()));
        assert_eq!(parse("/help").unwrap(), Command::Help(None));
        assert_eq!(parse("file metadata \"C:\\My Docs\\a.pdf\"").unwrap(), Command::FileMetadata("C:\\My Docs\\a.pdf".into()));
    }
    #[test]
    fn parses_case_and_report() {
        assert_eq!(parse("case open 12").unwrap(), Command::CaseOpen(12));
        assert_eq!(parse("case verify 3 off").unwrap(), Command::CaseVerify(3, false));
        assert_eq!(
            parse("report export --format csv").unwrap(),
            Command::ReportExport(Format::Csv, None)
        );
        assert!(parse("report export --format pdf").is_err());
        assert!(parse("report export").is_err());
    }
    #[test]
    fn rejects_bad_input() {
        assert!(parse("nonsense").is_err());
        assert!(parse("ip lookup").is_err());
        assert!(parse("case open abc").is_err());
        assert!(parse("echo \"unterminated").is_err());
        assert!(parse("").is_err());
    }
    #[test]
    fn completion_and_hint() {
        assert!(complete("dom").contains(&"domain scan "));
        assert!(hint("ip lookup 1.1.1.1").unwrap().contains("ip lookup <ip>"));
        assert_eq!(command_prefix_len("domain scan x.com"), Some(11));
    }
}
