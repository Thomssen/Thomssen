//! Thomsen Optimizer core: all platform-independent logic.
//!
//! Layout
//! * `sys`        - the Windows boundary (registry, power plans, windows, recycle bin)
//! * `journal`    - transactional change journal + restore
//! * `analysis`   - recommendation engine (pure)  /  `optimizer` - applies them
//! * `profiles`, `booster`, `games` - gaming profiles, auto-activation, game library
//! * `monitor`, `hwinfo` - live metrics and static hardware facts
//! * `cleaner`, `startup`, `procs` - cleanup, startup entries, process list
//! * `fps`, `history` - PresentMon capture, statistics, saved sessions
//! * `engine`     - ties everything together for the app shell

pub mod analysis;
pub mod booster;
pub mod cleaner;
pub mod engine;
pub mod error;
pub mod fps;
pub mod games;
pub mod history;
pub mod hwinfo;
pub mod journal;
pub mod logging;
pub mod monitor;
pub mod optimizer;
pub mod procs;
pub mod profiles;
pub mod settings;
pub mod startup;
pub mod store;
pub mod sys;

pub use error::{Error, Result};
