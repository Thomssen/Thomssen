// Types shared by the API and the admin dashboard. No runtime secrets here.
export type AccessStatus = "authorized" | "revoked" | "denied";
export type LoginResult =
  | "success" | "failed_membership" | "failed_role" | "failed_banned"
  | "failed_revoked" | "failed_oauth" | "failed_state" | "failed_admin_denied";

export interface AdminMe { discord_id: string; username: string; display_name: string | null; avatar_url: string | null; role: "admin" | "super_admin"; csrf: string }
export interface UserRow {
  id: string; discord_id: string; username: string; display_name: string | null; avatar_url: string | null;
  access_status: AccessStatus; banned: boolean; created_at: string; last_login_at: string | null; last_activity_at: string | null;
  last_login_ip: string | null; active_sessions: number;
}
export interface Overview {
  total_users: number; active_users_7d: number; valid_sessions: number; logins_today: number;
  failed_logins_today: number; cli_launches_today: number; banned_accounts: number;
  series: { day: string; success: number; failed: number }[];
}
export const ADMIN_ACTIONS = ["allow", "revoke", "ban", "unban", "revoke_sessions"] as const;
export type AdminAction = (typeof ADMIN_ACTIONS)[number];
