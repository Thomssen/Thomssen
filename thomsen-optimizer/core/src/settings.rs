//! Application settings (persisted as `settings.json`). Every field has a
//! default so older files keep loading after an update.

use crate::profiles::{default_profiles, ProfileConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // General
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub check_updates_automatically: bool,
    // Gaming
    pub auto_detect_games: bool,
    pub auto_boost: bool,
    pub default_profile_id: String,
    pub restore_when_game_closes: bool,
    // Monitoring
    /// Live refresh interval (ms) while a monitoring view is open.
    pub refresh_ms: u32,
    pub fps_monitoring: bool,
    pub history_retention_days: u32,
    pub temperature_monitoring: bool,
    // Optimization
    /// Confirm before applying anything that changes Windows settings.
    pub confirm_changes: bool,
    pub auto_restore_point: bool,
    /// Skip files modified within this many hours when cleaning.
    pub cleaner_min_age_hours: u32,
    // Appearance
    pub animations: bool,
    /// UI zoom factor, 0.8 - 1.3.
    pub ui_scale: f32,
    pub accent_color: String,
    // Paths
    pub presentmon_path: Option<String>,
    // Profiles (built-in ones are always present; users may edit them)
    pub profiles: Vec<ProfileConfig>,
    pub onboarding_completed: bool,
    pub last_seen_version: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_on_startup: false,
            minimize_to_tray: true,
            start_minimized: false,
            check_updates_automatically: true,
            auto_detect_games: true,
            auto_boost: true,
            default_profile_id: "balanced".into(),
            restore_when_game_closes: true,
            refresh_ms: 1000,
            fps_monitoring: false,
            history_retention_days: 90,
            temperature_monitoring: true,
            confirm_changes: true,
            auto_restore_point: false,
            cleaner_min_age_hours: 1,
            animations: true,
            ui_scale: 1.0,
            accent_color: "#ffffff".into(),
            presentmon_path: None,
            profiles: default_profiles(),
            onboarding_completed: false,
            last_seen_version: None,
        }
    }
}

impl Settings {
    /// Clamp/repair values after load or update, and make sure the three
    /// built-in profiles exist.
    pub fn sanitize(&mut self) {
        self.refresh_ms = self.refresh_ms.clamp(500, 10_000);
        self.history_retention_days = self.history_retention_days.clamp(1, 3650);
        self.cleaner_min_age_hours = self.cleaner_min_age_hours.min(24 * 365);
        self.ui_scale = if self.ui_scale.is_finite() { self.ui_scale.clamp(0.8, 1.3) } else { 1.0 };
        if !is_hex_color(&self.accent_color) {
            self.accent_color = "#ffffff".into();
        }
        for builtin in default_profiles() {
            if !self.profiles.iter().any(|p| p.id == builtin.id) {
                self.profiles.push(builtin);
            }
        }
        if !self.profiles.iter().any(|p| p.id == self.default_profile_id) {
            self.default_profile_id = "balanced".into();
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_repairs_values() {
        let mut s = Settings { refresh_ms: 5, ui_scale: f32::NAN, accent_color: "red".into(), default_profile_id: "gone".into(), ..Default::default() };
        s.profiles.clear();
        s.sanitize();
        assert_eq!(s.refresh_ms, 500);
        assert_eq!(s.ui_scale, 1.0);
        assert_eq!(s.accent_color, "#ffffff");
        assert_eq!(s.default_profile_id, "balanced");
        assert_eq!(s.profiles.len(), 3);
    }

    #[test]
    fn old_files_load_with_defaults() {
        let s: Settings = serde_json::from_str(r#"{"refreshMs":2000}"#).unwrap();
        assert_eq!(s.refresh_ms, 2000);
        assert!(s.minimize_to_tray);
    }
}
