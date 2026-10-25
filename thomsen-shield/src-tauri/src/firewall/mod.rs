pub mod elevated_ops;
pub mod ledger;
pub mod netfw;
pub mod worker;

use crate::error::{AppError, AppResult};
use crate::store::models::{FirewallRule, FirewallRuleAction, FirewallRuleDirection, FirewallRuleTargetType, FirewallStatus};

use elevated_ops::FirewallOp;

/// High-level firewall operations. Reads go straight to the in-process COM
/// worker thread (`worker::Handle`) - no elevation needed. Every write goes
/// through the ledger and an elevated helper invocation
/// (`elevated_ops::submit`) so the ledger and live Windows Firewall state
/// can never drift silently out of sync, and administrator rights are only
/// ever requested for the one action that actually needs them.
pub struct FirewallManager {
    worker: worker::Handle,
}

impl FirewallManager {
    pub fn new() -> Self {
        Self { worker: worker::Handle::spawn() }
    }

    pub async fn status(&self) -> AppResult<FirewallStatus> {
        self.worker.status().await
    }

    pub async fn create_ip_rule(&self, ip: &str, direction: FirewallRuleDirection) -> AppResult<FirewallRule> {
        let name = ledger::build_rule_name(FirewallRuleAction::Block, ip);
        let op = FirewallOp::Create {
            name: name.clone(),
            action: FirewallRuleAction::Block,
            direction,
            remote_addresses: Some(ip.to_string()),
            application_path: None,
        };
        submit_one(op).await?;

        let mut led = ledger::load()?;
        ledger::record(&mut led, name, FirewallRuleTargetType::Ip, ip.to_string(), direction, FirewallRuleAction::Block)
    }

    pub async fn create_app_rule(&self, app_path: &str, direction: FirewallRuleDirection) -> AppResult<FirewallRule> {
        let name = ledger::build_rule_name(FirewallRuleAction::Block, app_path);
        let op = FirewallOp::Create {
            name: name.clone(),
            action: FirewallRuleAction::Block,
            direction,
            remote_addresses: None,
            application_path: Some(app_path.to_string()),
        };
        submit_one(op).await?;

        let mut led = ledger::load()?;
        ledger::record(&mut led, name, FirewallRuleTargetType::Application, app_path.to_string(), direction, FirewallRuleAction::Block)
    }

    pub async fn remove_rule(&self, id: &str) -> AppResult<()> {
        let mut led = ledger::load()?;
        if let Some(rule) = led.rules.iter().find(|r| r.id == id).cloned() {
            submit_one(FirewallOp::Remove { name: rule.name }).await?;
            ledger::forget(&mut led, id)?;
        }
        Ok(())
    }

    pub async fn set_rule_enabled(&self, id: &str, enabled: bool) -> AppResult<()> {
        let mut led = ledger::load()?;
        if let Some(rule) = led.rules.iter_mut().find(|r| r.id == id) {
            submit_one(FirewallOp::SetEnabled { name: rule.name.clone(), enabled }).await?;
            rule.enabled = enabled;
            ledger::save(&led)?;
        }
        Ok(())
    }

    /// The ledger plus each rule's *live* enabled state, since it can be
    /// toggled from the Windows Firewall UI directly.
    pub async fn list_rules(&self) -> AppResult<Vec<FirewallRule>> {
        let led = ledger::load()?;
        let mut rules = Vec::with_capacity(led.rules.len());
        for mut rule in led.rules {
            if let Some(enabled) = self.worker.get_enabled(rule.name.clone()).await? {
                rule.enabled = enabled;
                rules.push(rule);
            }
            // else: gone from Windows Firewall - reconcile() is what
            // removes these from the ledger; list_rules stays read-mostly.
        }
        Ok(rules)
    }

    /// Removes every rule Thomsen Shield has ever created and clears the
    /// ledger - the "Reset Thomsen Shield Rules" safety net. One elevation
    /// prompt for the whole batch, not one per rule.
    pub async fn reset_all(&self) -> AppResult<usize> {
        let led = ledger::load()?;
        let count = led.rules.len();
        let ops = led.rules.iter().map(|r| FirewallOp::Remove { name: r.name.clone() }).collect();
        submit_batch(ops).await?;
        ledger::save(&ledger::Ledger::default())?;
        Ok(count)
    }

    /// Called once at startup: drops any ledger entry whose rule no longer
    /// exists in Windows Firewall (removed by hand). Read-only - never
    /// recreates anything, so a crash can never leave duplicate rules
    /// behind, and this never itself prompts for elevation.
    pub async fn reconcile_on_startup(&self) -> AppResult<()> {
        let mut led = ledger::load()?;
        let mut kept = Vec::with_capacity(led.rules.len());
        for rule in led.rules.drain(..) {
            if self.worker.get_enabled(rule.name.clone()).await?.is_some() {
                kept.push(rule);
            }
        }
        led.rules = kept;
        ledger::save(&led)
    }
}

impl Default for FirewallManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Elevation itself (spawning the helper, waiting on it while the UAC
/// prompt is up, reading its response file) is synchronous OS work that can
/// take as long as the user takes to respond - run on Tokio's blocking
/// thread pool so it never stalls the async runtime other commands (the
/// connection poll, other UI actions) still need to make progress on.
async fn submit_batch(ops: Vec<FirewallOp>) -> AppResult<()> {
    let response = tokio::task::spawn_blocking(move || elevated_ops::submit(ops))
        .await
        .map_err(|e| AppError::new(format!("The elevated action panicked: {e}")))??;
    if let Some(failed) = response.results.iter().find(|r| !r.ok) {
        return Err(AppError::new(failed.error.clone().unwrap_or_else(|| "The action failed.".to_string())));
    }
    Ok(())
}

async fn submit_one(op: FirewallOp) -> AppResult<()> {
    submit_batch(vec![op]).await
}
