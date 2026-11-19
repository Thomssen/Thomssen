//! Saved FPS sessions for before/after comparison. Local only.

use crate::error::{Error, Result};
use crate::fps::{downsample, stats, FrameStats};
use crate::store::DataDir;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

const FILE: &str = "sessions.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FpsSession {
    pub id: String,
    pub game_name: String,
    pub process_name: String,
    pub started_at: DateTime<Utc>,
    pub stats: FrameStats,
    /// ~500 averaged frame-time points for the graph.
    pub frame_times: Vec<f32>,
    /// User label such as "Before optimization" / "After optimization".
    pub label: String,
    /// Gaming profile active while this was captured (`None` = none).
    pub profile_id: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub a: FpsSession,
    pub b: FpsSession,
    /// Percent change B vs A of the measured value (`None` when either side lacks it).
    pub avg_fps_change_pct: Option<f64>,
    pub low_fps_change_pct: Option<f64>,
    pub stability_change_pts: f64,
    pub same_game: bool,
    /// Always shown next to a comparison.
    pub disclaimer: String,
}

fn pct(a: f64, b: f64) -> Option<f64> {
    (a > 0.0).then(|| (b - a) / a * 100.0)
}

pub fn compare(a: FpsSession, b: FpsSession) -> Comparison {
    Comparison {
        avg_fps_change_pct: pct(a.stats.avg_fps, b.stats.avg_fps),
        low_fps_change_pct: a.stats.one_percent_low_fps.zip(b.stats.one_percent_low_fps).and_then(|(x, y)| pct(x, y)),
        stability_change_pts: b.stats.stability_pct - a.stats.stability_pct,
        same_game: a.process_name.eq_ignore_ascii_case(&b.process_name),
        disclaimer: "These are measurements of two separate sessions. Differences can come from the scene, game updates, temperatures or background load, not only from optimizations - repeat the same benchmark pass several times for a trustworthy comparison.".into(),
        a,
        b,
    }
}

pub struct History {
    data: DataDir,
}

impl History {
    pub fn new(data: DataDir) -> Self {
        Self { data }
    }

    pub fn list(&self) -> Vec<FpsSession> {
        let mut v: Vec<FpsSession> = self.data.read_json(FILE);
        v.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        v
    }

    pub fn add(&self, game: &str, process: &str, started_at: DateTime<Utc>, frames: &[f32], label: &str, profile_id: Option<String>, retention_days: u32) -> Result<FpsSession> {
        let st = stats(frames).ok_or_else(|| Error::Invalid("No frames were captured, so there is nothing to save.".into()))?;
        let s = FpsSession { id: uuid::Uuid::new_v4().to_string(), game_name: game.into(), process_name: process.into(), started_at, stats: st, frame_times: downsample(frames, 500), label: label.chars().take(60).collect(), profile_id, notes: String::new() };
        let mut all = self.list();
        all.insert(0, s.clone());
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        all.retain(|x| x.started_at >= cutoff);
        self.data.write_json(FILE, &all)?;
        Ok(s)
    }

    pub fn get(&self, id: &str) -> Result<FpsSession> {
        self.list().into_iter().find(|s| s.id == id).ok_or_else(|| Error::NotFound("Session not found.".into()))
    }

    pub fn update(&self, id: &str, label: &str, notes: &str) -> Result<()> {
        let mut all = self.list();
        let s = all.iter_mut().find(|s| s.id == id).ok_or_else(|| Error::NotFound("Session not found.".into()))?;
        s.label = label.chars().take(60).collect();
        s.notes = notes.chars().take(500).collect();
        self.data.write_json(FILE, &all)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let mut all = self.list();
        all.retain(|s| s.id != id);
        self.data.write_json(FILE, &all)
    }

    pub fn clear(&self) -> Result<()> {
        self.data.write_json(FILE, &Vec::<FpsSession>::new())
    }

    pub fn prune(&self, retention_days: u32) {
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let all: Vec<FpsSession> = self.list().into_iter().filter(|s| s.started_at >= cutoff).collect();
        let _ = self.data.write_json(FILE, &all);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_compare_and_retention() {
        let d = tempfile::tempdir().unwrap();
        let h = History::new(DataDir::new(d.path()).unwrap());
        assert!(h.add("G", "g.exe", Utc::now(), &[], "x", None, 30).is_err());
        let a = h.add("G", "g.exe", Utc::now(), &vec![10.0; 300], "Before optimization", None, 30).unwrap();
        let b = h.add("G", "g.exe", Utc::now(), &vec![8.0; 300], "After optimization", Some("competitive".into()), 30).unwrap();
        let c = compare(a.clone(), b);
        assert!((c.avg_fps_change_pct.unwrap() - 25.0).abs() < 1e-9);
        assert!(c.same_game && c.disclaimer.contains("measurements"));
        // Old sessions fall out of the retention window.
        h.add("G", "g.exe", Utc::now() - Duration::days(100), &vec![10.0; 120], "old", None, 3650).unwrap();
        h.prune(30);
        assert_eq!(h.list().len(), 2);
        h.update(&a.id, "renamed", "notes").unwrap();
        assert_eq!(h.get(&a.id).unwrap().label, "renamed");
        h.delete(&a.id).unwrap();
        assert_eq!(h.list().len(), 1);
    }
}
