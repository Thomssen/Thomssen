//! Direct COM interop with Windows Firewall (`INetFwPolicy2`/`INetFwRule`) -
//! the same API `netsh advfirewall` itself is built on. Used instead of
//! shelling out to `netsh` so rule state is read as structured data rather
//! than parsed from locale-dependent verbose text output.
//!
//! Every function here must only ever be called from `firewall::worker`'s
//! dedicated OS thread - COM objects created on one apartment-threaded
//! thread cannot be safely used from another without marshaling, so this
//! module deliberately never spawns its own thread or crosses an `.await`.

use windows::core::BSTR;
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::NetworkManagement::WindowsFirewall::{
    INetFwPolicy2, INetFwRule, NetFwPolicy2, NetFwRule, NET_FW_ACTION_ALLOW, NET_FW_ACTION_BLOCK,
    NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE, NET_FW_PROFILE2_PUBLIC, NET_FW_RULE_DIR_IN,
    NET_FW_RULE_DIR_OUT,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

use crate::error::{AppError, AppResult};
use crate::store::models::{FirewallProfilesEnabled, FirewallRuleAction, FirewallRuleDirection, FirewallStatus};

fn policy() -> AppResult<INetFwPolicy2> {
    unsafe { CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| AppError::new(format!("Couldn't reach Windows Firewall: {e}")))
}

pub fn status() -> AppResult<FirewallStatus> {
    let policy = policy()?;
    let domain = unsafe { policy.get_FirewallEnabled(NET_FW_PROFILE2_DOMAIN) }.unwrap_or(VARIANT_BOOL(0)).as_bool();
    let private = unsafe { policy.get_FirewallEnabled(NET_FW_PROFILE2_PRIVATE) }.unwrap_or(VARIANT_BOOL(0)).as_bool();
    let public = unsafe { policy.get_FirewallEnabled(NET_FW_PROFILE2_PUBLIC) }.unwrap_or(VARIANT_BOOL(0)).as_bool();
    Ok(FirewallStatus {
        active: domain || private || public,
        profiles_enabled: FirewallProfilesEnabled { domain, private, public },
    })
}

pub struct NewRule<'a> {
    pub name: &'a str,
    pub action: FirewallRuleAction,
    pub direction: FirewallRuleDirection,
    /// Exactly one of these is set by the caller - see `create_ip_rule` /
    /// `create_app_rule` in `firewall::mod`.
    pub remote_addresses: Option<&'a str>,
    pub application_path: Option<&'a str>,
}

/// Creates and registers a new rule. `name` should already carry the
/// `ThomsenShield_` prefix - this module doesn't enforce naming, the
/// ledger layer does.
pub fn create_rule(spec: NewRule<'_>) -> AppResult<()> {
    let policy = policy()?;
    let rule: INetFwRule = unsafe { CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| AppError::new(format!("Couldn't create the firewall rule: {e}")))?;

    unsafe {
        rule.SetName(&BSTR::from(spec.name)).ok();
        rule.SetEnabled(VARIANT_BOOL::from(true)).ok();
        rule.SetAction(match spec.action {
            FirewallRuleAction::Block => NET_FW_ACTION_BLOCK,
            FirewallRuleAction::Allow => NET_FW_ACTION_ALLOW,
        })
        .ok();
        rule.SetDirection(match spec.direction {
            FirewallRuleDirection::In => NET_FW_RULE_DIR_IN,
            FirewallRuleDirection::Out => NET_FW_RULE_DIR_OUT,
        })
        .ok();
        if let Some(remote) = spec.remote_addresses {
            rule.SetRemoteAddresses(&BSTR::from(remote))
                .map_err(|e| AppError::new(format!("Invalid remote address: {e}")))?;
        }
        if let Some(app) = spec.application_path {
            rule.SetApplicationName(&BSTR::from(app))
                .map_err(|e| AppError::new(format!("Invalid application path: {e}")))?;
        }

        let rules = policy.Rules().map_err(|e| AppError::new(format!("Couldn't reach Windows Firewall: {e}")))?;
        rules.Add(&rule).map_err(|e| {
            AppError::new(format!(
                "Windows Firewall refused this rule. Thomsen Shield needs to run as administrator for this action: {e}"
            ))
        })?;
    }
    Ok(())
}

pub fn remove_rule(name: &str) -> AppResult<()> {
    let policy = policy()?;
    let rules = unsafe { policy.Rules() }.map_err(|e| AppError::new(format!("Couldn't reach Windows Firewall: {e}")))?;
    // Removing a rule that no longer exists is not an error here - the
    // reconciliation pass and Reset both rely on this being idempotent.
    match unsafe { rules.Remove(&BSTR::from(name)) } {
        Ok(()) => Ok(()),
        Err(e) if is_not_found(&e) => Ok(()),
        Err(e) => Err(AppError::new(format!(
            "Windows Firewall refused to remove this rule. Thomsen Shield needs to run as administrator for this action: {e}"
        ))),
    }
}

/// `Ok(None)` means the rule no longer exists (removed outside Thomsen
/// Shield, e.g. by hand in the Windows Firewall UI).
pub fn get_enabled(name: &str) -> AppResult<Option<bool>> {
    let policy = policy()?;
    let rules = unsafe { policy.Rules() }.map_err(|e| AppError::new(format!("Couldn't reach Windows Firewall: {e}")))?;
    match unsafe { rules.Item(&BSTR::from(name)) } {
        Ok(rule) => {
            let enabled = unsafe { rule.Enabled() }.map_err(|e| AppError::new(format!("Couldn't read rule state: {e}")))?;
            Ok(Some(enabled.as_bool()))
        }
        Err(e) if is_not_found(&e) => Ok(None),
        Err(e) => Err(AppError::new(format!("Couldn't reach Windows Firewall: {e}"))),
    }
}

pub fn set_enabled(name: &str, enabled: bool) -> AppResult<()> {
    let policy = policy()?;
    let rules = unsafe { policy.Rules() }.map_err(|e| AppError::new(format!("Couldn't reach Windows Firewall: {e}")))?;
    let rule = unsafe { rules.Item(&BSTR::from(name)) }
        .map_err(|e| AppError::new(format!("That rule no longer exists in Windows Firewall: {e}")))?;
    unsafe { rule.SetEnabled(VARIANT_BOOL::from(enabled)) }
        .map_err(|e| AppError::new(format!("Windows Firewall refused this change. Thomsen Shield needs to run as administrator for this action: {e}")))
}

fn is_not_found(e: &windows::core::Error) -> bool {
    // HRESULT_FROM_WIN32(ERROR_NOT_FOUND) / E_NOT_FOUND-equivalent Windows
    // Firewall raises when a named rule doesn't exist.
    const HRESULT_ELEMENT_NOT_FOUND: i32 = -2147024809; // 0x80070490
    e.code().0 == HRESULT_ELEMENT_NOT_FOUND
}
