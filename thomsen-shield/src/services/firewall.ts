import { invoke } from "./ipc";
import type { FirewallRule, FirewallRuleDirection, FirewallStatus } from "@/types";

export function getFirewallStatus(): Promise<FirewallStatus> {
  return invoke<FirewallStatus>("get_firewall_status");
}

export function listFirewallRules(): Promise<FirewallRule[]> {
  return invoke<FirewallRule[]>("list_firewall_rules");
}

export function createIpBlockRule(ip: string, direction: FirewallRuleDirection = "in"): Promise<FirewallRule> {
  return invoke<FirewallRule>("create_ip_block_rule", { ip, direction });
}

export function createAppBlockRule(path: string, direction: FirewallRuleDirection = "out"): Promise<FirewallRule> {
  return invoke<FirewallRule>("create_app_block_rule", { path, direction });
}

export function removeFirewallRule(id: string): Promise<void> {
  return invoke<void>("remove_firewall_rule", { id });
}

export function setFirewallRuleEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke<void>("set_firewall_rule_enabled", { id, enabled });
}

export function resetThomsenShieldRules(): Promise<{ removed: number }> {
  return invoke<{ removed: number }>("reset_thomsen_shield_rules");
}
