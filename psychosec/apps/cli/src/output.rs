//! Structured module output. Every module returns an `Output`, which is rendered in
//! the terminal, can be saved to a case as a finding, and exported in reports.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Err,
    Info,
    Dim,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Row {
    Heading { text: String },
    Kv { key: String, value: String },
    Text { text: String },
    Item { status: Status, label: String, detail: String },
    Blank,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Output {
    pub module: String,
    pub target: String,
    pub title: String,
    pub rows: Vec<Row>,
    pub sources: Vec<String>,
    pub collected_at: String,
}

impl Output {
    pub fn new(module: &str, target: &str, title: &str) -> Self {
        Self {
            module: module.into(),
            target: target.into(),
            title: title.into(),
            rows: vec![],
            sources: vec![],
            collected_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        }
    }
    pub fn heading(&mut self, t: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Blank);
        self.rows.push(Row::Heading { text: t.into() });
        self
    }
    pub fn kv(&mut self, k: impl Into<String>, v: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Kv { key: k.into(), value: v.into() });
        self
    }
    pub fn text(&mut self, t: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Text { text: t.into() });
        self
    }
    pub fn item(&mut self, s: Status, label: impl Into<String>, detail: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Item { status: s, label: label.into(), detail: detail.into() });
        self
    }
    pub fn source(&mut self, url: impl Into<String>) -> &mut Self {
        let u = url.into();
        if !self.sources.contains(&u) {
            self.sources.push(u);
        }
        self
    }
    /// Plain-text rendering used by CSV/HTML summaries.
    pub fn plain_lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter_map(|r| match r {
                Row::Heading { text } => Some(format!("[{text}]")),
                Row::Kv { key, value } => Some(format!("{key}: {value}")),
                Row::Text { text } => Some(text.clone()),
                Row::Item { status, label, detail } => Some(format!("{:?} {label} {detail}", status).trim().to_string()),
                Row::Blank => None,
            })
            .collect()
    }
}

/// A single line in the terminal scrollback.
#[derive(Clone, Debug)]
pub enum OutLine {
    Blank,
    Cmd(String),
    Title(String),
    Heading(String),
    Kv(String, String),
    Text(String),
    Item(Status, String, String),
    Notice(Status, String),
}

impl Output {
    pub fn to_lines(&self) -> Vec<OutLine> {
        let mut v = vec![OutLine::Blank, OutLine::Title(self.title.clone())];
        for r in &self.rows {
            v.push(match r {
                Row::Blank => OutLine::Blank,
                Row::Heading { text } => OutLine::Heading(text.clone()),
                Row::Kv { key, value } => OutLine::Kv(key.clone(), value.clone()),
                Row::Text { text } => OutLine::Text(text.clone()),
                Row::Item { status, label, detail } => OutLine::Item(*status, label.clone(), detail.clone()),
            });
        }
        if !self.sources.is_empty() {
            v.push(OutLine::Blank);
            v.push(OutLine::Heading("Sources".into()));
            for s in &self.sources {
                v.push(OutLine::Text(s.clone()));
            }
        }
        v.push(OutLine::Blank);
        v
    }
}
