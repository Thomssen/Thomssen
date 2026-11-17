//! Platform-independent core of Privacy Blur.
//!
//! Nothing in this crate touches the screen, the network or the disk. It takes
//! text plus pixel rectangles in, and returns *categories plus rectangles* out.
//! Matched text is never stored in any returned structure, so it cannot leak into
//! logs or IPC messages by accident.

pub mod detect;
pub mod effects;
pub mod entries;
pub mod geometry;
pub mod locate;
pub mod settings;
pub mod track;

pub use detect::{Detector, Hit};
pub use entries::{Entry, EntryKind};
pub use geometry::Rect;
pub use settings::{Category, Settings};
