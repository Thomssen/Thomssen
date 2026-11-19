//! In-memory [`SystemOps`] used by unit tests (and available to integration
//! tests). It can be told to fail specific operations to exercise rollback.

use super::*;
use std::collections::HashMap;
use std::sync::Mutex;

type Key = (Hive, String, String);

#[derive(Default)]
pub struct FakeOps {
    reg: Mutex<HashMap<Key, (String, RegData)>>,
    plans: Mutex<Vec<PowerPlan>>,
    /// Registry writes to a value whose name contains this string fail.
    pub fail_name: Mutex<Option<String>>,
}

fn key(h: Hive, p: &str, n: &str) -> Key {
    (h, p.to_ascii_lowercase(), n.to_ascii_lowercase())
}

impl FakeOps {
    pub fn with_default_plans() -> Self {
        let f = Self::default();
        *f.plans.lock().unwrap() = vec![
            PowerPlan { guid: GUID_BALANCED.into(), name: "Balanced".into(), active: true, kind: PlanKind::Balanced },
            PowerPlan { guid: GUID_HIGH.into(), name: "High performance".into(), active: false, kind: PlanKind::HighPerformance },
            PowerPlan { guid: GUID_SAVER.into(), name: "Power saver".into(), active: false, kind: PlanKind::PowerSaver },
        ];
        f
    }
    pub fn fail_writes_to(&self, name: &str) {
        *self.fail_name.lock().unwrap() = Some(name.to_string());
    }
    pub fn active_guid(&self) -> Option<String> {
        self.plans.lock().unwrap().iter().find(|p| p.active).map(|p| p.guid.clone())
    }
}

impl SystemOps for FakeOps {
    fn reg_get(&self, h: Hive, p: &str, n: &str) -> Result<Option<RegData>> {
        Ok(self.reg.lock().unwrap().get(&key(h, p, n)).map(|(_, d)| d.clone()))
    }
    fn reg_set(&self, h: Hive, p: &str, n: &str, d: &RegData) -> Result<()> {
        if let Some(f) = self.fail_name.lock().unwrap().as_ref() {
            if n.contains(f.as_str()) {
                return Err(Error::PermissionDenied(format!("simulated failure writing {n}")));
            }
        }
        self.reg.lock().unwrap().insert(key(h, p, n), (n.to_string(), d.clone()));
        Ok(())
    }
    fn reg_delete_value(&self, h: Hive, p: &str, n: &str) -> Result<()> {
        self.reg.lock().unwrap().remove(&key(h, p, n));
        Ok(())
    }
    fn reg_values(&self, h: Hive, p: &str) -> Result<Vec<(String, RegData)>> {
        let lp = p.to_ascii_lowercase();
        let mut v: Vec<_> = self.reg.lock().unwrap().iter().filter(|((hh, pp, _), _)| *hh == h && *pp == lp).map(|(_, (n, d))| (n.clone(), d.clone())).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(v)
    }
    fn reg_subkeys(&self, _: Hive, _: &str) -> Result<Vec<String>> {
        Ok(vec![])
    }
    fn power_plans(&self) -> Result<Vec<PowerPlan>> {
        Ok(self.plans.lock().unwrap().clone())
    }
    fn power_set_active(&self, guid: &str) -> Result<()> {
        if !valid_guid(guid) {
            return Err(Error::Invalid("not a valid power plan GUID".into()));
        }
        let mut plans = self.plans.lock().unwrap();
        if !plans.iter().any(|p| p.guid.eq_ignore_ascii_case(guid)) {
            return Err(Error::NotFound("power plan not found".into()));
        }
        for p in plans.iter_mut() {
            p.active = p.guid.eq_ignore_ascii_case(guid);
        }
        Ok(())
    }
    fn power_ensure_ultimate(&self) -> Result<PowerPlan> {
        let mut plans = self.plans.lock().unwrap();
        if let Some(p) = plans.iter().find(|p| p.kind == PlanKind::Ultimate) {
            return Ok(p.clone());
        }
        let p = PowerPlan { guid: "11111111-2222-3333-4444-555555555555".into(), name: format!("Ultimate Performance {ULTIMATE_MARKER}"), active: false, kind: PlanKind::Ultimate };
        plans.push(p.clone());
        Ok(p)
    }
}
