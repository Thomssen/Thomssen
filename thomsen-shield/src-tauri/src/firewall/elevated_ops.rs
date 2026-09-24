//! The write side of firewall management. Creating, removing, or toggling
//! a rule always requires administrator rights on Windows, so every one of
//! these goes through `elevation::run_elevated`, which re-launches this
//! same executable elevated for exactly this one batch of work and exits.
//!
//! Batched (`Vec<FirewallOp>`) deliberately: "Reset Thomsen Shield Rules"
//! removing a dozen rules must be one UAC prompt, not a dozen.

use serde::{Deserialize, Serialize};

use crate::elevation;
use crate::error::AppResult;
use crate::store::models::{FirewallRuleAction, FirewallRuleDirection};

use super::netfw::{self, NewRule};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirewallOp {
    Create {
        name: String,
        action: FirewallRuleAction,
        direction: FirewallRuleDirection,
        remote_addresses: Option<String>,
        application_path: Option<String>,
    },
    Remove {
        name: String,
    },
    SetEnabled {
        name: String,
        enabled: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElevatedRequest {
    pub ops: Vec<FirewallOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpResult {
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElevatedResponse {
    pub results: Vec<OpResult>,
}

/// Parent side: prompts for elevation once and applies every op in order.
/// One failed op does not stop the rest - `reset_all` in particular wants
/// "remove everything you can" even if one rule was already gone.
pub fn submit(ops: Vec<FirewallOp>) -> AppResult<ElevatedResponse> {
    if ops.is_empty() {
        return Ok(ElevatedResponse { results: Vec::new() });
    }
    let request = ElevatedRequest { ops };
    elevation::run_elevated(&request)
}

/// Child side: called from `main.rs`'s early-exit path when this process
/// was re-launched with `--elevated-op`. Runs on this process's own main
/// thread, which is fine - this invocation does nothing else and exits
/// immediately after.
pub fn run_elevated_child() -> ! {
    let exit_code = match run_elevated_child_inner() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("thomsen-shield elevated helper: {e}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run_elevated_child_inner() -> AppResult<()> {
    let (request, out_path): (ElevatedRequest, _) = elevation::parse_elevated_request()?;

    unsafe {
        windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED)
    }
    .ok()
    .map_err(|e| crate::error::AppError::new(format!("Couldn't initialize Windows Firewall access: {e}")))?;

    let results: Vec<OpResult> = request
        .ops
        .into_iter()
        .map(|op| match apply(op) {
            Ok(()) => OpResult { ok: true, error: None },
            Err(e) => OpResult { ok: false, error: Some(e.message) },
        })
        .collect();

    unsafe { windows::Win32::System::Com::CoUninitialize() };

    elevation::write_elevated_response(&out_path, &ElevatedResponse { results })
}

fn apply(op: FirewallOp) -> AppResult<()> {
    match op {
        FirewallOp::Create { name, action, direction, remote_addresses, application_path } => netfw::create_rule(NewRule {
            name: &name,
            action,
            direction,
            remote_addresses: remote_addresses.as_deref(),
            application_path: application_path.as_deref(),
        }),
        FirewallOp::Remove { name } => netfw::remove_rule(&name),
        FirewallOp::SetEnabled { name, enabled } => netfw::set_enabled(&name, enabled),
    }
}
