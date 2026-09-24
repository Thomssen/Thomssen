//! The authoritative record of every firewall rule Thomsen Shield has ever
//! created. This is what makes "Reset Thomsen Shield Rules" and crash-safe
//! startup reconciliation possible - Windows Firewall itself is never
//! trusted as the only source of truth for "did we create this."

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::store::{load_or_default, paths, save_atomic};
use crate::store::models::{FirewallRule, FirewallRuleAction, FirewallRuleDirection, FirewallRuleTargetType};

pub const RULE_PREFIX: &str = "ThomsenShield_";

/// Builds `ThomsenShield_Block_<target>` (or similar), sanitized so the
/// result is always a legal, single-line Windows Firewall rule name - see
/// `sanitize_for_rule_name`.
pub fn build_rule_name(action: FirewallRuleAction, target: &str) -> String {
    let verb = match action {
        FirewallRuleAction::Block => "Block",
        FirewallRuleAction::Allow => "Allow",
    };
    format!("{RULE_PREFIX}{verb}_{}", sanitize_for_rule_name(target))
}

fn sanitize_for_rule_name(target: &str) -> String {
    target
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    pub rules: Vec<FirewallRule>,
}

pub fn load() -> AppResult<Ledger> {
    load_or_default(&paths::rules_ledger_path()?)
}

pub fn save(ledger: &Ledger) -> AppResult<()> {
    save_atomic(&paths::rules_ledger_path()?, ledger)
}

pub fn record(ledger: &mut Ledger, name: String, target_type: FirewallRuleTargetType, target: String, direction: FirewallRuleDirection, action: FirewallRuleAction) -> AppResult<FirewallRule> {
    let rule = FirewallRule {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        target_type,
        target,
        direction,
        action,
        enabled: true,
        created_at: chrono::Utc::now(),
    };
    ledger.rules.push(rule.clone());
    save(ledger)?;
    Ok(rule)
}

pub fn forget(ledger: &mut Ledger, id: &str) -> AppResult<()> {
    ledger.rules.retain(|r| r.id != id);
    save(ledger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_readable_ip_rule_names() {
        assert_eq!(build_rule_name(FirewallRuleAction::Block, "185.34.22.10"), "ThomsenShield_Block_185.34.22.10");
    }

    #[test]
    fn sanitizes_application_paths_for_the_rule_name() {
        let name = build_rule_name(FirewallRuleAction::Block, r"C:\Program Files\App\app.exe");
        assert_eq!(name, "ThomsenShield_Block_C__Program_Files_App_app.exe");
        // No path separators, spaces, or other characters netsh/the
        // Firewall UI would choke on in a rule name.
        assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'));
    }
}
