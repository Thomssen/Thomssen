use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Email,
    Name,
    Address,
    Phone,
    Username,
    Keyword,
    Regex,
}

/// A piece of personal information the user typed in. Stored DPAPI-encrypted on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub kind: EntryKind,
    pub value: String,
}

pub const MAX_ENTRY_LEN: usize = 500;

/// Validate user input before it is stored. Returns a user-presentable message on failure.
pub fn validate(kind: EntryKind, value: &str) -> Result<(), String> {
    let v = value.trim();
    if v.is_empty() {
        return Err("Value must not be empty.".into());
    }
    if v.chars().count() > MAX_ENTRY_LEN {
        return Err(format!("Value is too long (max {MAX_ENTRY_LEN} characters)."));
    }
    match kind {
        EntryKind::Regex => {
            RegexBuilder::new(v)
                .size_limit(2 << 20)
                .build()
                .map_err(|e| format!("Invalid pattern: {e}"))?;
            if RegexBuilder::new(v).build().map(|r| r.is_match("")).unwrap_or(false) {
                return Err("Pattern matches empty text and would blur everything.".into());
            }
        }
        EntryKind::Email => {
            if !v.contains('@') {
                return Err("An email address must contain '@'.".into());
            }
        }
        EntryKind::Phone => {
            if v.chars().filter(|c| c.is_ascii_digit()).count() < 6 {
                return Err("A phone number needs at least 6 digits.".into());
            }
        }
        _ => {
            if v.chars().count() < 2 {
                return Err("Value is too short.".into());
            }
        }
    }
    Ok(())
}
