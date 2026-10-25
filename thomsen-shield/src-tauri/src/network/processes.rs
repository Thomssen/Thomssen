use std::collections::HashMap;

use sysinfo::{Pid, System};

/// Thin cache over `sysinfo::System` so connection polling doesn't refresh
/// the whole process list on every single lookup - refreshed once per
/// `connections::snapshot()` call instead.
pub struct ProcessLookup {
    system: System,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub exe_path: Option<String>,
}

impl ProcessLookup {
    pub fn new() -> Self {
        Self { system: System::new() }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    }

    pub fn lookup(&self, pid: u32) -> Option<ProcessInfo> {
        let process = self.system.process(Pid::from_u32(pid))?;
        Some(ProcessInfo {
            name: process.name().to_string_lossy().into_owned(),
            exe_path: process.exe().map(|p| p.to_string_lossy().into_owned()),
        })
    }

    /// Bulk variant so `connections::snapshot()` only walks the process
    /// table once per poll instead of once per connection.
    pub fn snapshot(&self) -> HashMap<u32, ProcessInfo> {
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| {
                (
                    pid.as_u32(),
                    ProcessInfo {
                        name: process.name().to_string_lossy().into_owned(),
                        exe_path: process.exe().map(|p| p.to_string_lossy().into_owned()),
                    },
                )
            })
            .collect()
    }
}

impl Default for ProcessLookup {
    fn default() -> Self {
        Self::new()
    }
}
