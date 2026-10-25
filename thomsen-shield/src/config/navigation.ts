/**
 * Sidebar navigation. Text only - no icons - to keep the rail quiet, same
 * convention as every other Thomsen app.
 *
 * Add a page: add a `Route`, an entry in `NAV_GROUPS` here, and a case in
 * <RouteView> (src/app/RouteView.tsx).
 */

export type Route =
  | "dashboard"
  | "connections"
  | "protection"
  | "firewall"
  | "blocklist"
  | "allowlist"
  | "logs"
  | "settings";

export interface NavItem {
  route: Route;
  label: string;
}

/** Primary nav. Settings is pinned separately at the rail's foot. */
export const NAV_GROUPS: readonly (readonly NavItem[])[] = [
  [
    { route: "dashboard", label: "Dashboard" },
    { route: "connections", label: "Live Connections" },
  ],
  [
    { route: "protection", label: "Protection" },
    { route: "firewall", label: "Firewall" },
    { route: "blocklist", label: "Blocklist" },
    { route: "allowlist", label: "Allowlist" },
  ],
  [{ route: "logs", label: "Events & Logs" }],
] as const;

export const SETTINGS_ITEM: NavItem = { route: "settings", label: "Settings" };

export const ROUTE_LABELS: Record<Route, string> = Object.fromEntries(
  [...NAV_GROUPS.flat(), SETTINGS_ITEM].map((i) => [i.route, i.label]),
) as Record<Route, string>;

export const DEFAULT_ROUTE: Route = "dashboard";
