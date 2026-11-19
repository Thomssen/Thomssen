/**
 * Sidebar navigation. Text only - no icons - to keep the rail quiet, same
 * convention as every other Thomsen app.
 *
 * Add a page: add a `Route`, an entry in `NAV_GROUPS` here, and a case in
 * <RouteView> (src/app/RouteView.tsx).
 */

export type Route =
  | "dashboard"
  | "optimize"
  | "game-booster"
  | "games"
  | "performance"
  | "cleaner"
  | "startup"
  | "system"
  | "settings"
  | "results";

export interface NavItem {
  route: Route;
  label: string;
}

export const NAV_GROUPS: readonly (readonly NavItem[])[] = [
  [{ route: "dashboard", label: "Dashboard" }],
  [
    { route: "optimize", label: "Optimize" },
    { route: "game-booster", label: "Game Booster" },
    { route: "games", label: "Games" },
    { route: "performance", label: "Performance" },
  ],
  [
    { route: "cleaner", label: "System Cleaner" },
    { route: "startup", label: "Startup Manager" },
    { route: "system", label: "System Information" },
  ],
] as const;

export const SETTINGS_ITEM: NavItem = { route: "settings", label: "Settings" };

export const ROUTE_LABELS: Record<Route, string> = {
  ...(Object.fromEntries([...NAV_GROUPS.flat(), SETTINGS_ITEM].map((i) => [i.route, i.label])) as Record<Exclude<Route, "results">, string>),
  results: "Optimization Results",
};

/** Pages that show live graphs: the monitor runs at full refresh rate only here. */
export const MONITORING_ROUTES: readonly Route[] = ["dashboard", "performance", "game-booster"];

export const DEFAULT_ROUTE: Route = "dashboard";
