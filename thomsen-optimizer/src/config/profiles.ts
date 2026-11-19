import type { ProfileConfig } from "@/types";

/** Factory defaults for the three built-in profiles. Mirrors `default_profiles()`
 * in core/src/profiles.rs (used by "Reset to default" in the profile editor). */
export const DEFAULT_PROFILES: ProfileConfig[] = [
  {
    id: "balanced",
    name: "Balanced",
    description: "Minimal changes and low system impact. Keeps your current power plan and only makes sure Windows Game Mode is on.",
    builtin: true,
    power: "keep",
    gameMode: true,
    disableGameDvr: false,
    closeApps: [],
    tradeoffs: "Very little to gain or lose; suitable for everyday use.",
  },
  {
    id: "competitive",
    name: "Competitive",
    description: "Aims for consistent frame times: High performance power plan, Game Mode on, Xbox Game Bar background recording off.",
    builtin: true,
    power: "highPerformance",
    gameMode: true,
    disableGameDvr: true,
    closeApps: [],
    tradeoffs: "Slightly higher power use and heat than Balanced; background game clip recording is unavailable while active.",
  },
  {
    id: "maximum",
    name: "Maximum Performance",
    description: "Uses the Ultimate Performance power plan where available (otherwise High performance), plus everything in Competitive.",
    builtin: true,
    power: "ultimate",
    gameMode: true,
    disableGameDvr: true,
    closeApps: [],
    tradeoffs: "Highest power consumption and heat; shorter battery life on laptops and louder fans. The CPU stays at high clocks even in menus.",
  },
];

export const POWER_LABELS: Record<string, string> = {
  keep: "Keep current plan",
  balanced: "Balanced",
  highPerformance: "High performance",
  ultimate: "Ultimate Performance",
};
