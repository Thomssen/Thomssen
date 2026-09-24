//! The background monitoring loop: polls active connections, feeds them to
//! the detection engine, acts on what it flags, and sweeps expired
//! temporary blocks. Runs on its own interval independent of anything the
//! UI does, so the UI stays responsive and this keeps working even with no
//! window open.

use std::net::Ipv4Addr;
use std::sync::Arc;

use crate::detection::Verdict;
use crate::netutil::{ip_matches, is_reserved_local};
use crate::store::models::{BlockReason, BlockStatus, BlockedEntry, FirewallRuleDirection, LogCategory, ProtectionMode, Settings};
use crate::store::{allowlist, blocklist, settings as settings_store};
use crate::{logging, network, state::AppState};

const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3);

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = tick(&state).await {
                // A single bad poll (e.g. a transient permissions hiccup
                // enumerating one process) must never kill the loop -
                // log it and keep going.
                let _ = logging::append(LogCategory::Warning, format!("Monitoring cycle failed: {e}"));
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

async fn tick(state: &AppState) -> crate::error::AppResult<()> {
    let settings = settings_store::load()?;
    let allowlist_values: Vec<String> = allowlist::list()?.into_iter().map(|e| e.value).collect();

    sweep_expired_blocks(state).await?;

    if !settings.protection.enabled {
        return Ok(());
    }

    let suspicious_before = state.detection.lock().await.suspicious_ips();
    let connections = network::connections::snapshot(&allowlist_values, &suspicious_before)?;

    {
        let mut points = state.activity_points.lock().await;
        points.push_back(connections.len() as u32);
        while points.len() > crate::state::ACTIVITY_HISTORY_LEN {
            points.pop_front();
        }
    }

    let ids_and_ips: Vec<(String, Ipv4Addr)> = connections
        .iter()
        .filter_map(|c| c.remote_address.parse::<Ipv4Addr>().ok().map(|ip| (c.id.clone(), ip)))
        .collect();

    // Lockdown Mode escalates to the strictest sensitivity and forces
    // auto-blocking on for the duration it's active - "block new suspicious
    // remote sources quickly" - but it never overrides Monitoring Only's
    // own, stronger guarantee that it blocks nothing automatically; see
    // `handle_verdict`. Allowlisted/local traffic is unaffected either way,
    // since `is_protected` below is checked before anything else.
    let effective_sensitivity = if settings.lockdown_mode {
        crate::store::models::SensitivityLevel::High
    } else {
        settings.protection.sensitivity
    };

    let verdicts: Vec<Verdict> = {
        let mut engine = state.detection.lock().await;
        engine.observe(
            ids_and_ips.iter().map(|(id, ip)| (id.as_str(), *ip)),
            chrono::Utc::now(),
            effective_sensitivity,
            settings.protection.detect_connection_spikes,
            settings.protection.detect_repeated_connections,
            |ip| is_reserved_local(ip) || allowlist_values.iter().any(|e| ip_matches(ip, e)),
        )
    };

    for verdict in verdicts {
        handle_verdict(state, &settings, verdict).await?;
    }

    Ok(())
}

async fn handle_verdict(state: &AppState, settings: &Settings, verdict: Verdict) -> crate::error::AppResult<()> {
    let reason_label = match verdict.reason {
        BlockReason::ConnectionSpike => "Suspicious connection rate detected",
        BlockReason::RepeatedConnections => "Repeated connections detected",
        _ => "Suspicious activity detected",
    };
    logging::append(LogCategory::Warning, format!("{reason_label} from {}", verdict.ip))?;

    // ConnectionSpike is a best-guess attribution across *all* processes and
    // IPs in the window (whichever contributed most - see detection::engine)
    // rather than one address alone crossing its own threshold, so a burst
    // caused by several unrelated ordinary apps (a VPN reconnect, several
    // apps waking at once) can point at an innocent bystander. It's always
    // logged above; only RepeatedConnections (one IP alone, on its own
    // merits) is trusted enough to auto-block.
    if verdict.reason == BlockReason::ConnectionSpike {
        return Ok(());
    }

    // Monitoring Only never blocks, regardless of the auto-block toggle or
    // Lockdown Mode - it's the one mode the spec guarantees never acts on
    // its own. Lockdown otherwise forces blocking on even if the user left
    // auto-block off, since that is the entire point of turning it on.
    let auto_block_wanted = settings.protection.auto_block_suspicious_ips || settings.lockdown_mode;
    let should_block = auto_block_wanted && settings.protection_mode != ProtectionMode::MonitoringOnly;
    if !should_block {
        return Ok(());
    }

    let expires_at = settings.protection.block_duration.to_chrono_duration().map(|d| chrono::Utc::now() + d);

    // Create the real rule *before* recording the block - the reverse order
    // let a failed/declined elevation still leave the blocklist (and so the
    // UI) claiming this IP was blocked when no firewall rule exists at all.
    match state.firewall.create_ip_rule(&verdict.ip.to_string(), FirewallRuleDirection::In).await {
        Ok(_) => {
            let entry = BlockedEntry {
                id: uuid::Uuid::new_v4().to_string(),
                ip: verdict.ip.to_string(),
                reason: verdict.reason,
                note: None,
                created_at: chrono::Utc::now(),
                expires_at,
                status: BlockStatus::Active,
            };
            blocklist::add(entry)?;
            let duration_label = expires_at.map(|_| "temporarily").unwrap_or("permanently");
            logging::append(LogCategory::Blocked, format!("{} blocked {duration_label}", verdict.ip))?;
        }
        Err(e) => {
            logging::append(LogCategory::Warning, format!("Couldn't create a firewall rule for {}: {e}", verdict.ip))?;
        }
    }

    Ok(())
}

async fn sweep_expired_blocks(state: &AppState) -> crate::error::AppResult<()> {
    let now = chrono::Utc::now();
    let entries = blocklist::list()?;
    for mut entry in entries {
        if entry.status == BlockStatus::Active {
            if let Some(expires_at) = entry.expires_at {
                if expires_at <= now {
                    let ip = entry.ip.clone();
                    // Remove the live rule *before* marking the entry
                    // Expired - the reverse order let a failed removal (a
                    // declined/timed-out elevation prompt, a transient COM
                    // error) get reported as "expired" while the firewall
                    // rule stayed in place. On failure, just leave the
                    // entry Active and retry on the next poll (3s) rather
                    // than persisting a status the removal never confirmed.
                    let removed = match find_rule_id_for_ip(state, &ip).await? {
                        Some(rule_id) => state.firewall.remove_rule(&rule_id).await.is_ok(),
                        None => true, // no live rule to remove - already gone
                    };
                    if !removed {
                        logging::append(LogCategory::Warning, format!("Couldn't remove the expired firewall rule for {ip} yet - will retry"))?;
                        continue;
                    }
                    entry.status = BlockStatus::Expired;
                    blocklist::update(entry)?;
                    state.detection.lock().await.clear_flag(ip.parse().unwrap_or(Ipv4Addr::UNSPECIFIED));
                    logging::append(LogCategory::Blocked, format!("Block expired for {ip}"))?;
                }
            }
        }
    }
    Ok(())
}

async fn find_rule_id_for_ip(state: &AppState, ip: &str) -> crate::error::AppResult<Option<String>> {
    let rules = state.firewall.list_rules().await?;
    Ok(rules.into_iter().find(|r| r.target == ip).map(|r| r.id))
}
