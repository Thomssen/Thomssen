//! Gaming profiles. Three built-ins (Balanced / Competitive / Maximum
//! Performance) that the user may edit, plus any number of custom profiles.
//!
//! A profile only ever contains settings Thomsen Optimizer can apply AND
//! reverse (power plan, Game Mode, Game DVR background capture). Nothing in
//! here promises an FPS gain - the effect depends on the system and is
//! measured on the Performance page.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PowerChoice {
    /// Leave the active plan alone.
    Keep,
    Balanced,
    HighPerformance,
    /// "Ultimate Performance" if the system has / can create it, otherwise
    /// falls back to High performance.
    Ultimate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub builtin: bool,
    pub power: PowerChoice,
    /// Make sure Windows Game Mode is on.
    pub game_mode: bool,
    /// Turn off Xbox Game Bar background recording while gaming.
    pub disable_game_dvr: bool,
    /// Executable names (e.g. `OneDrive.exe`) the user wants closed when this
    /// profile is activated **manually**. Automatic activation never closes
    /// anything with a visible window - see `booster`.
    pub close_apps: Vec<String>,
    /// Plain-language downsides shown next to the profile.
    pub tradeoffs: String,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Custom".into(),
            description: String::new(),
            builtin: false,
            power: PowerChoice::Keep,
            game_mode: true,
            disable_game_dvr: false,
            close_apps: vec![],
            tradeoffs: String::new(),
        }
    }
}

pub fn default_profiles() -> Vec<ProfileConfig> {
    vec![
        ProfileConfig {
            id: "balanced".into(),
            name: "Balanced".into(),
            description: "Minimal changes and low system impact. Keeps your current power plan and only makes sure Windows Game Mode is on.".into(),
            builtin: true,
            power: PowerChoice::Keep,
            game_mode: true,
            disable_game_dvr: false,
            close_apps: vec![],
            tradeoffs: "Very little to gain or lose; suitable for everyday use.".into(),
        },
        ProfileConfig {
            id: "competitive".into(),
            name: "Competitive".into(),
            description: "Aims for consistent frame times: High performance power plan, Game Mode on, Xbox Game Bar background recording off.".into(),
            builtin: true,
            power: PowerChoice::HighPerformance,
            game_mode: true,
            disable_game_dvr: true,
            close_apps: vec![],
            tradeoffs: "Slightly higher power use and heat than Balanced; background game clip recording is unavailable while active.".into(),
        },
        ProfileConfig {
            id: "maximum".into(),
            name: "Maximum Performance".into(),
            description: "Uses the Ultimate Performance power plan where available (otherwise High performance), plus everything in Competitive.".into(),
            builtin: true,
            power: PowerChoice::Ultimate,
            game_mode: true,
            disable_game_dvr: true,
            close_apps: vec![],
            tradeoffs: "Highest power consumption and heat; shorter battery life on laptops and louder fans. The CPU stays at high clocks even in menus.".into(),
        },
    ]
}

pub fn find<'a>(profiles: &'a [ProfileConfig], id: &str) -> Option<&'a ProfileConfig> {
    profiles.iter().find(|p| p.id == id)
}

/// Executable names are stored and later compared case-insensitively. Keep
/// them to plain file names so a crafted profile can never smuggle in a path
/// or command line.
pub fn valid_exe_name(s: &str) -> bool {
    let l = s.len();
    (5..=96).contains(&l)
        && s.to_ascii_lowercase().ends_with(".exe")
        && !s.contains(['/', '\\', ':', '"', '<', '>', '|', '*', '?'])
        && !s.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_name_validation() {
        assert!(valid_exe_name("OneDrive.exe"));
        assert!(!valid_exe_name("..\\evil.exe"));
        assert!(!valid_exe_name("C:\\a.exe"));
        assert!(!valid_exe_name("notanexe.txt"));
        assert!(!valid_exe_name("a b|c.exe"));
    }

    #[test]
    fn builtins_present() {
        let p = default_profiles();
        assert_eq!(p.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["balanced", "competitive", "maximum"]);
    }
}
