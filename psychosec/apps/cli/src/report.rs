//! Case report export: HTML, JSON, CSV.
use crate::commands::Format;
use crate::output::Row;
use crate::store::{Case, Finding};
use std::path::{Path, PathBuf};

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// CSV field with RFC 4180 quoting and spreadsheet formula-injection neutralisation.
pub fn csv_field(s: &str) -> String {
    let mut v = s.replace('\r', " ");
    if matches!(v.chars().next(), Some('=' | '+' | '-' | '@' | '\t')) {
        v.insert(0, '\'');
    }
    if v.contains([',', '"', '\n']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v
    }
}

pub fn render_json(case: &Case, findings: &[Finding]) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "generator": format!("PSYCHOSEC {}", crate::config::VERSION),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "case": case,
        "findings": findings,
    }))
    .map_err(|e| e.to_string())
}

pub fn render_csv(case: &Case, findings: &[Finding]) -> String {
    let mut s = String::from("case_id,case_name,finding_id,module,target,title,verified,collected_at,source_urls,note,summary\n");
    for f in findings {
        let summary = f.data.plain_lines().join(" | ");
        let cols = [
            case.id.to_string(),
            case.name.clone(),
            f.id.to_string(),
            f.module.clone(),
            f.target.clone(),
            f.title.clone(),
            if f.verified { "verified" } else { "unverified" }.to_string(),
            f.collected_at.clone(),
            f.source_url.replace('\n', " "),
            f.note.clone(),
            summary,
        ];
        s.push_str(&cols.iter().map(|c| csv_field(c)).collect::<Vec<_>>().join(","));
        s.push_str("\r\n");
    }
    s
}

pub fn render_html(case: &Case, findings: &[Finding]) -> String {
    let e = html_escape;
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    h.push_str(&format!("<title>{} - PSYCHOSEC report</title>", e(&case.name)));
    h.push_str("<style>body{background:#08090B;color:#F2F2F2;font:14px/1.55 ui-monospace,Consolas,monospace;max-width:920px;margin:40px auto;padding:0 20px}h1{font-size:20px;letter-spacing:.2em}h2{font-size:15px;color:#87BDBB;border-bottom:1px solid #292B2F;padding-bottom:6px;margin-top:36px}.dim{color:#858585}.f{border:1px solid #292B2F;border-radius:6px;padding:14px 18px;margin:16px 0}table{border-collapse:collapse;width:100%}td{padding:2px 12px 2px 0;vertical-align:top}td:first-child{color:#858585;white-space:nowrap}.ok{color:#91C5A5}.err{color:#D78585}.warn{color:#D6BF8A}.tag{font-size:12px;border:1px solid #292B2F;border-radius:4px;padding:1px 6px}</style></head><body>");
    h.push_str("<h1>PSYCHOSEC</h1><div class=\"dim\">PRIVATE OSINT &amp; INVESTIGATION TERMINAL &middot; case report</div>");
    h.push_str(&format!("<h2>{}</h2>", e(&case.name)));
    h.push_str(&format!(
        "<table><tr><td>Case ID</td><td>{}</td></tr><tr><td>Created</td><td>{}</td></tr><tr><td>Exported</td><td>{}</td></tr><tr><td>Findings</td><td>{}</td></tr></table>",
        case.id, e(&case.created_at), chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"), findings.len()
    ));
    if !case.targets.is_empty() {
        h.push_str("<h2>Targets</h2><ul>");
        for t in &case.targets {
            h.push_str(&format!("<li>{}</li>", e(t)));
        }
        h.push_str("</ul>");
    }
    if !case.notes.is_empty() {
        h.push_str("<h2>Notes</h2><ul>");
        for t in &case.notes {
            h.push_str(&format!("<li>{}</li>", e(t)));
        }
        h.push_str("</ul>");
    }
    h.push_str("<h2>Findings</h2>");
    for f in findings {
        h.push_str(&format!(
            "<div class=\"f\"><b>#{} {}</b> <span class=\"tag {}\">{}</span><div class=\"dim\">{} &middot; collected {}</div>",
            f.id, e(&f.title), if f.verified { "ok" } else { "warn" }, if f.verified { "verified" } else { "unverified" }, e(&f.module), e(&f.collected_at)
        ));
        if !f.note.is_empty() {
            h.push_str(&format!("<p>Note: {}</p>", e(&f.note)));
        }
        h.push_str("<table>");
        for r in &f.data.rows {
            match r {
                Row::Heading { text } => h.push_str(&format!("<tr><td colspan=2 style=\"padding-top:10px;color:#87BDBB\">{}</td></tr>", e(text))),
                Row::Kv { key, value } => h.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>", e(key), e(value))),
                Row::Text { text } => h.push_str(&format!("<tr><td colspan=2>{}</td></tr>", e(text))),
                Row::Item { status, label, detail } => {
                    let c = match status {
                        crate::output::Status::Ok => "ok",
                        crate::output::Status::Err => "err",
                        crate::output::Status::Warn => "warn",
                        _ => "dim",
                    };
                    h.push_str(&format!("<tr><td class=\"{c}\">{}</td><td>{}</td></tr>", e(label), e(detail)));
                }
                Row::Blank => {}
            }
        }
        h.push_str("</table>");
        for s in f.source_url.lines().filter(|l| !l.is_empty()) {
            h.push_str(&format!("<div class=\"dim\">source: {}</div>", e(s)));
        }
        h.push_str("</div>");
    }
    h.push_str("<p class=\"dim\">Generated locally by PSYCHOSEC. Results reflect public data at the collection time shown and may be incomplete or inaccurate; verify before relying on them.</p></body></html>");
    h
}

pub fn export(case: &Case, findings: &[Finding], fmt: &Format, out: Option<&str>) -> Result<PathBuf, String> {
    let ext = match fmt {
        Format::Html => "html",
        Format::Json => "json",
        Format::Csv => "csv",
    };
    let body = match fmt {
        Format::Html => render_html(case, findings),
        Format::Json => render_json(case, findings)?,
        Format::Csv => render_csv(case, findings),
    };
    let path = match out {
        Some(p) => PathBuf::from(p),
        None => crate::config::reports_dir().join(format!("case-{}-{}.{}", case.id, chrono::Utc::now().format("%Y%m%d-%H%M%S"), ext)),
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    write(&path, &body)?;
    Ok(path)
}

fn write(path: &Path, body: &str) -> Result<(), String> {
    std::fs::write(path, body).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{Output, Status};

    fn sample() -> (Case, Vec<Finding>) {
        let mut o = Output::new("web", "https://x", "Web <script>");
        o.kv("Server", "nginx").item(Status::Warn, "CSP", "missing");
        (
            Case { id: 1, name: "Case \"A\"".into(), notes: vec!["n<1>".into()], targets: vec!["x".into()], created_at: "t".into(), updated_at: "t".into() },
            vec![Finding { id: 5, case_id: 1, module: "web".into(), target: "x".into(), title: "<b>T</b>".into(), source_url: "https://x".into(), collected_at: "t".into(), verified: true, note: "=cmd()".into(), data: o }],
        )
    }

    #[test]
    fn html_is_escaped() {
        let (c, f) = sample();
        let h = render_html(&c, &f);
        assert!(!h.contains("<b>T</b>"));
        assert!(h.contains("&lt;b&gt;T&lt;/b&gt;"));
        assert!(h.contains("n&lt;1&gt;"));
    }
    #[test]
    fn csv_quotes_and_neutralises_formulas() {
        let (c, f) = sample();
        let s = render_csv(&c, &f);
        assert!(s.contains("'=cmd()"));
        assert!(s.contains("\"Case \"\"A\"\"\""));
        assert_eq!(csv_field("plain"), "plain");
    }
    #[test]
    fn json_roundtrips() {
        let (c, f) = sample();
        let v: serde_json::Value = serde_json::from_str(&render_json(&c, &f).unwrap()).unwrap();
        assert_eq!(v["findings"][0]["id"], 5);
    }
}
