//! Pure detection logic - no I/O, no OS calls, so it's fully unit-testable
//! and the thresholds are easy to see and tune in one place.
//!
//! Deliberately keyed on *new* connection establishment, not raw presence:
//! a single long-lived connection (a voice call, a game session) is one
//! event when it starts and then never again, so it can never itself look
//! like "repeated connections" no matter how long it stays open. Only a
//! real burst of distinct new connection attempts - from one IP, or across
//! many - trips either signal. That asymmetry is what keeps ordinary apps
//! quiet; see the spec's own warning against over-aggressive detection.

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::Ipv4Addr;

use chrono::{DateTime, Utc};

use crate::store::models::{BlockReason, SensitivityLevel};

const SPIKE_WINDOW_SECS: i64 = 10;
const REPEAT_WINDOW_SECS: i64 = 60;

// Raised well above ordinary bursty apps after a false-positive audit found
// the original values (60/35/18 and 40/20/10) tripped on totally normal
// behavior: restoring 40+ browser tabs at once can plausibly open 100-160+
// new connections in a 10s window (many third-party origins per page, even
// with HTTP/2 multiplexing), and a large Steam/launcher download routinely
// opens 20-30+ new connections per minute to one CDN edge. These numbers
// are a starting point, not a measured ceiling - they're deliberately
// generous until there's real telemetry to tune against, since blocking a
// paying customer's browser or game client on day one is far costlier than
// a slightly slower reaction to genuine abuse.
fn spike_threshold(sensitivity: SensitivityLevel) -> usize {
    match sensitivity {
        SensitivityLevel::Low => 350,
        SensitivityLevel::Normal => 200,
        SensitivityLevel::High => 100,
    }
}

fn repeat_threshold(sensitivity: SensitivityLevel) -> usize {
    match sensitivity {
        SensitivityLevel::Low => 120,
        SensitivityLevel::Normal => 60,
        SensitivityLevel::High => 30,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    pub ip: Ipv4Addr,
    pub reason: BlockReason,
}

pub struct DetectionEngine {
    known_ids: HashSet<String>,
    /// False only until the very first `observe()` call completes. Without
    /// this, every currently-open socket on the machine at the moment
    /// Thomsen Shield (re)starts - a totally ordinary state, since the user
    /// almost certainly already has a browser/Discord/game running - would
    /// have no prior `known_ids` entry and so would count as "new" all at
    /// once, guaranteed-tripping the spike threshold on the very first poll
    /// after every launch or reboot. Priming just learns what's already
    /// there before counting anything as new.
    primed: bool,
    new_connections_by_ip: HashMap<Ipv4Addr, VecDeque<DateTime<Utc>>>,
    new_connection_times: VecDeque<DateTime<Utc>>,
    /// IPs already flagged and not yet cleared - suppresses re-flagging
    /// (and so re-blocking) the same address every single poll while it
    /// stays active.
    flagged: HashSet<Ipv4Addr>,
}

impl DetectionEngine {
    pub fn new() -> Self {
        Self {
            known_ids: HashSet::new(),
            primed: false,
            new_connections_by_ip: HashMap::new(),
            new_connection_times: VecDeque::new(),
            flagged: HashSet::new(),
        }
    }

    pub fn suspicious_ips(&self) -> HashSet<Ipv4Addr> {
        self.flagged.clone()
    }

    pub fn clear_flag(&mut self, ip: Ipv4Addr) {
        self.flagged.remove(&ip);
    }

    /// Feed one poll's worth of active connections. `protected` (allowlist
    /// + the always-on reserved local ranges) is checked before anything
    /// else - a protected IP can never be flagged, full stop, regardless of
    /// how it behaves. `detect_spikes`/`detect_repeats` mirror the user's
    /// Protection settings toggles of the same name - false suppresses that
    /// specific verdict without touching the other one.
    pub fn observe<'a>(
        &mut self,
        active: impl Iterator<Item = (&'a str, Ipv4Addr)>,
        now: DateTime<Utc>,
        sensitivity: SensitivityLevel,
        detect_spikes: bool,
        detect_repeats: bool,
        is_protected: impl Fn(Ipv4Addr) -> bool,
    ) -> Vec<Verdict> {
        let mut current_ids = HashSet::new();
        let mut new_ips = Vec::new();

        for (id, remote_ip) in active {
            current_ids.insert(id.to_string());
            if remote_ip.is_unspecified() {
                continue; // UDP rows with no real peer - see network::connections.
            }
            if self.primed && !self.known_ids.contains(id) {
                new_ips.push(remote_ip);
            }
        }
        self.known_ids = current_ids;

        if !self.primed {
            self.primed = true;
            return Vec::new();
        }

        let spike_cutoff = now - chrono::Duration::seconds(SPIKE_WINDOW_SECS);
        let repeat_cutoff = now - chrono::Duration::seconds(REPEAT_WINDOW_SECS);

        let mut verdicts = Vec::new();

        for ip in new_ips {
            self.new_connection_times.push_back(now);
            self.new_connections_by_ip.entry(ip).or_default().push_back(now);
        }

        while matches!(self.new_connection_times.front(), Some(t) if *t < spike_cutoff) {
            self.new_connection_times.pop_front();
        }

        for deque in self.new_connections_by_ip.values_mut() {
            while matches!(deque.front(), Some(t) if *t < repeat_cutoff) {
                deque.pop_front();
            }
        }
        self.new_connections_by_ip.retain(|_, deque| !deque.is_empty());

        // Spike: too many *distinct* new connections across all IPs in the
        // short window - attribute it to whichever IP contributed the most,
        // since that is almost always the actual source.
        if detect_spikes && self.new_connection_times.len() > spike_threshold(sensitivity) {
            if let Some((&ip, _)) = self.new_connections_by_ip.iter().max_by_key(|(_, d)| d.len()) {
                if !is_protected(ip) && self.flagged.insert(ip) {
                    verdicts.push(Verdict { ip, reason: BlockReason::ConnectionSpike });
                }
            }
        }

        // Repeated connections: one IP alone crossing its own threshold.
        if detect_repeats {
            for (&ip, deque) in &self.new_connections_by_ip {
                if deque.len() > repeat_threshold(sensitivity) && !is_protected(ip) && self.flagged.insert(ip) {
                    verdicts.push(Verdict { ip, reason: BlockReason::RepeatedConnections });
                }
            }
        }

        verdicts
    }
}

impl Default for DetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
        Ipv4Addr::new(a, b, c, d)
    }

    #[test]
    fn a_single_long_lived_connection_never_flags() {
        let mut engine = DetectionEngine::new();
        let remote = ip(51, 137, 137, 111);
        let start = Utc::now();

        // Same connection id observed on every poll for two minutes - one
        // "new" event at t=0 (suppressed anyway, since it's the priming
        // tick), then nothing new ever again.
        for i in 0..40 {
            let now = start + chrono::Duration::seconds(i * 3);
            let verdicts = engine.observe(std::iter::once(("conn-1", remote)), now, SensitivityLevel::Normal, true, true, |_| false);
            assert!(verdicts.is_empty(), "flagged a single stable connection at tick {i}");
        }
    }

    #[test]
    fn a_burst_of_pre_existing_connections_on_first_observe_never_flags() {
        // Simulates launching Thomsen Shield (or rebooting with it
        // starting automatically) while a browser, Discord, and a couple
        // of other ordinary apps are already holding dozens of established
        // connections - the exact scenario the priming fix targets.
        let mut engine = DetectionEngine::new();
        let start = Utc::now();
        let pre_existing: Vec<(String, Ipv4Addr)> = (0..80).map(|i| (format!("pre-existing-{i}"), ip(93, 184, 220, (i % 250) as u8))).collect();

        let verdicts = engine.observe(pre_existing.iter().map(|(id, ip)| (id.as_str(), *ip)), start, SensitivityLevel::High, true, true, |_| false);
        assert!(verdicts.is_empty(), "flagged on the priming tick");

        // Detection must still work normally on every tick after the first -
        // enough distinct new connections here to clear High's spike
        // threshold on its own, well above the pre-existing 80 that must
        // stay uncounted.
        let now = start + chrono::Duration::seconds(3);
        let genuinely_new: Vec<(String, Ipv4Addr)> = (0..150).map(|i| (format!("new-{i}"), ip(198, 51, (i / 250) as u8, (i % 250) as u8))).collect();
        let mut all_pre_existing_and_new = pre_existing.clone();
        all_pre_existing_and_new.extend(genuinely_new);
        let verdicts = engine.observe(all_pre_existing_and_new.iter().map(|(id, ip)| (id.as_str(), *ip)), now, SensitivityLevel::High, true, true, |_| false);
        assert!(!verdicts.is_empty(), "priming permanently disabled detection instead of just skipping tick one");
    }

    #[test]
    fn rapid_new_connections_from_one_ip_trip_repeated_detection() {
        let mut engine = DetectionEngine::new();
        let remote = ip(185, 34, 22, 10);
        let start = Utc::now();
        let mut all_verdicts = Vec::new();

        // 65 distinct new connections from the same IP inside 60s - above
        // Normal's repeat_threshold of 60.
        for i in 0..65 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            all_verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, true, |_| false));
        }

        assert_eq!(all_verdicts, vec![Verdict { ip: remote, reason: BlockReason::RepeatedConnections }]);
    }

    #[test]
    fn moderate_bursty_traffic_from_one_ip_stays_quiet() {
        // A browser opening ~10 short-lived connections to the same CDN IP
        // inside a minute must never trip Normal sensitivity.
        let mut engine = DetectionEngine::new();
        let remote = ip(142, 250, 80, 14);
        let start = Utc::now();
        let mut all_verdicts = Vec::new();

        for i in 0..10 {
            let now = start + chrono::Duration::seconds(i * 4);
            let id = format!("conn-{i}");
            all_verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, true, |_| false));
        }

        assert!(all_verdicts.is_empty());
    }

    #[test]
    fn a_forty_tab_restore_stays_quiet_at_normal_sensitivity() {
        // ~160 new connections across many distinct CDN/ad/analytics IPs in
        // a couple of seconds - a plausible reading of restoring ~40 tabs
        // at once - must not trip the default sensitivity.
        let mut engine = DetectionEngine::new();
        let start = Utc::now();
        // Priming tick, empty.
        engine.observe(std::iter::empty(), start, SensitivityLevel::Normal, true, true, |_| false);

        let now = start + chrono::Duration::seconds(2);
        let tabs: Vec<(String, Ipv4Addr)> = (0..160).map(|i| (format!("tab-conn-{i}"), ip(93, 184, (i / 250) as u8, (i % 250) as u8))).collect();
        let verdicts = engine.observe(tabs.iter().map(|(id, ip)| (id.as_str(), *ip)), now, SensitivityLevel::Normal, true, true, |_| false);
        assert!(verdicts.is_empty(), "a normal tab-restore burst tripped detection: {verdicts:?}");
    }

    #[test]
    fn allowlisted_ip_is_never_flagged_even_when_abusive() {
        let mut engine = DetectionEngine::new();
        let remote = ip(10, 0, 0, 5);
        let start = Utc::now();
        let mut all_verdicts = Vec::new();

        for i in 0..65 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            all_verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, true, |_| true));
        }

        assert!(all_verdicts.is_empty());
    }

    #[test]
    fn an_ip_is_only_flagged_once_until_cleared() {
        let mut engine = DetectionEngine::new();
        let remote = ip(185, 34, 22, 10);
        let start = Utc::now();
        let mut all_verdicts = Vec::new();

        for i in 0..65 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            all_verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, true, |_| false));
        }
        // Keep pushing more new connections from the same, already-flagged
        // IP - must not produce a second verdict.
        for i in 65..75 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            all_verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, true, |_| false));
        }

        assert_eq!(all_verdicts.len(), 1);
        assert!(engine.suspicious_ips().contains(&remote));
    }

    #[test]
    fn disabling_a_detector_suppresses_only_that_verdict() {
        let remote = ip(185, 34, 22, 11);
        let start = Utc::now();

        // Repeat detection disabled: the same burst that trips it above must now stay quiet.
        let mut engine = DetectionEngine::new();
        let mut verdicts = Vec::new();
        for i in 0..65 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            verdicts.extend(engine.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Normal, true, false, |_| false));
        }
        assert!(verdicts.is_empty(), "RepeatedConnections fired while detect_repeats was false");
    }

    #[test]
    fn high_sensitivity_flags_sooner_than_low() {
        let start = Utc::now();
        let remote = ip(203, 0, 113, 9);

        let mut low = DetectionEngine::new();
        let mut low_flagged = false;
        for i in 0..35 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            if !low.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::Low, true, true, |_| false).is_empty() {
                low_flagged = true;
            }
        }
        assert!(!low_flagged, "Low sensitivity flagged after only 35 connections");

        let mut high = DetectionEngine::new();
        let mut high_flagged = false;
        for i in 0..35 {
            let now = start + chrono::Duration::seconds(i);
            let id = format!("conn-{i}");
            if !high.observe(std::iter::once((id.as_str(), remote)), now, SensitivityLevel::High, true, true, |_| false).is_empty() {
                high_flagged = true;
            }
        }
        assert!(high_flagged, "High sensitivity failed to flag 35 connections in 35s");
    }
}
