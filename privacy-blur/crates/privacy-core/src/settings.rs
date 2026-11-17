use serde::{Deserialize, Serialize};

/// Detection categories produced by the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Email,
    Phone,
    Name,
    Address,
    Ip,
    CreditCard,
    Bank,
    Username,
    Dob,
    Secret,
    Custom,
}

impl Category {
    pub const ALL: [Category; 11] = [
        Category::Email,
        Category::Phone,
        Category::Name,
        Category::Address,
        Category::Ip,
        Category::CreditCard,
        Category::Bank,
        Category::Username,
        Category::Dob,
        Category::Secret,
        Category::Custom,
    ];
}

/// One toggle per entry in the Privacy Filters page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub emails: bool,
    pub phones: bool,
    pub names: bool,
    pub addresses: bool,
    pub ips: bool,
    /// credit cards, IBANs and bank account numbers
    pub financial: bool,
    pub usernames: bool,
    pub dob: bool,
    /// passwords, tokens and API keys
    pub passwords: bool,
    /// custom keywords / patterns
    pub custom: bool,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            emails: true,
            phones: true,
            names: true,
            addresses: true,
            ips: true,
            financial: true,
            usernames: true,
            dob: true,
            passwords: true,
            custom: true,
        }
    }
}

impl Filters {
    pub fn enabled(&self, c: Category) -> bool {
        match c {
            Category::Email => self.emails,
            Category::Phone => self.phones,
            Category::Name => self.names,
            Category::Address => self.addresses,
            Category::Ip => self.ips,
            Category::CreditCard | Category::Bank => self.financial,
            Category::Username => self.usernames,
            Category::Dob => self.dob,
            Category::Secret => self.passwords,
            Category::Custom => self.custom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Gaussian,
    Pixelate,
    Solid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub style: Style,
    /// 0..=100, scales the Gaussian radius between 1px and `blur_radius`.
    pub intensity: u8,
    /// Maximum Gaussian radius in pixels.
    pub blur_radius: u32,
    /// Pixelation block size in pixels.
    pub pixel_size: u32,
    /// 20..=100. Below 100 the content underneath stays partly visible.
    pub opacity: u8,
    /// Extra pixels covered around each detected region.
    pub padding: u32,
    /// Never let the blur be weaker than what is needed to make the text unreadable.
    pub safe_minimum: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            style: Style::Gaussian,
            intensity: 80,
            blur_radius: 14,
            pixel_size: 10,
            opacity: 100,
            padding: 3,
            safe_minimum: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    All,
    Selected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppRules {
    pub mode: AppMode,
    /// lower-case executable names, e.g. "discord.exe"
    pub selected: Vec<String>,
    pub excluded: Vec<String>,
    pub protect_browsers: bool,
    /// Also censor Privacy Blur's own window.
    pub protect_self: bool,
}

impl Default for AppRules {
    fn default() -> Self {
        Self {
            mode: AppMode::All,
            selected: vec![],
            excluded: vec![],
            protect_browsers: true,
            protect_self: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Hybrid,
    UiaOnly,
    OcrOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    AllWindows,
    ForegroundOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Performance {
    /// Minimum time between capture/detection cycles.
    pub interval_ms: u32,
    pub method: Method,
    pub scope: Scope,
    /// OCR input scale in percent (50..=100). Lower is faster but misses small text.
    pub ocr_scale: u32,
    /// How long a region stays after its text was last seen (prevents flicker).
    pub hold_ms: u32,
}

impl Default for Performance {
    fn default() -> Self {
        Self {
            interval_ms: 300,
            method: Method::Hybrid,
            scope: Scope::AllWindows,
            ocr_scale: 100,
            hold_ms: 700,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub autostart: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub protect_on_launch: bool,
}

impl Default for General {
    fn default() -> Self {
        Self { autostart: false, start_minimized: false, minimize_to_tray: true, protect_on_launch: false }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub toggle: String,
    pub open_settings: String,
    pub panic: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            toggle: "Ctrl+Shift+B".into(),
            open_settings: "Ctrl+Shift+P".into(),
            panic: "Ctrl+Shift+X".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub on_toggle: bool,
    pub on_panic: bool,
    pub on_error: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Self { on_toggle: true, on_panic: true, on_error: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureSettings {
    /// true  → the overlay is an ordinary window, so display-capture based tools
    ///         (OBS "Display Capture", Discord "Screen") record it, i.e. record the blur.
    /// false → the overlay is excluded from every capture (WDA_EXCLUDEFROMCAPTURE): only
    ///         you see the blur and recordings/streams show the original content.
    pub overlay_in_captures: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self { overlay_in_captures: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Monitors {
    /// Device names (e.g. `\\.\DISPLAY2`) that are NOT protected.
    pub disabled: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub filters: Filters,
    pub appearance: Appearance,
    pub apps: AppRules,
    pub performance: Performance,
    pub general: General,
    pub shortcuts: Shortcuts,
    pub notifications: Notifications,
    pub capture: CaptureSettings,
    pub monitors: Monitors,
}

fn norm_list(v: &mut Vec<String>) {
    for s in v.iter_mut() {
        *s = s.trim().to_lowercase();
    }
    v.retain(|s| !s.is_empty() && s.len() <= 260);
    v.sort();
    v.dedup();
    v.truncate(500);
}

impl Settings {
    /// Clamp everything to its valid range. Called on every settings write from the UI.
    pub fn sanitize(&mut self) {
        let a = &mut self.appearance;
        a.intensity = a.intensity.min(100);
        a.blur_radius = a.blur_radius.clamp(2, 80);
        a.pixel_size = a.pixel_size.clamp(3, 60);
        a.opacity = a.opacity.clamp(20, 100);
        a.padding = a.padding.min(24);
        let p = &mut self.performance;
        p.interval_ms = p.interval_ms.clamp(100, 3000);
        p.ocr_scale = p.ocr_scale.clamp(40, 100);
        p.hold_ms = p.hold_ms.clamp(0, 5000);
        norm_list(&mut self.apps.selected);
        norm_list(&mut self.apps.excluded);
        self.monitors.disabled.truncate(32);
        for s in [&mut self.shortcuts.toggle, &mut self.shortcuts.open_settings, &mut self.shortcuts.panic] {
            s.truncate(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sanitize_clamps_and_normalises() {
        let mut s = Settings::default();
        s.appearance.intensity = 250;
        s.appearance.opacity = 0;
        s.performance.interval_ms = 1;
        s.apps.excluded = vec!["  Chrome.EXE ".into(), "chrome.exe".into(), "".into()];
        s.sanitize();
        assert_eq!(s.appearance.intensity, 100);
        assert_eq!(s.appearance.opacity, 20);
        assert_eq!(s.performance.interval_ms, 100);
        assert_eq!(s.apps.excluded, vec!["chrome.exe"]);
    }
    #[test]
    fn partial_json_gets_defaults() {
        let s: Settings = serde_json::from_str(r#"{"filters":{"emails":false}}"#).unwrap();
        assert!(!s.filters.emails && s.filters.phones);
    }
}
