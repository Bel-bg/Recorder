//! Domain layer of Recoder: recordings, local storage and statistics.
//!
//! This crate has no UI and no Android dependency so it can be exercised
//! with plain `cargo test`.

pub mod error;
pub mod format;
pub mod models;
pub mod stats;
pub mod storage;

pub use error::{Error, Result};
pub use models::Recording;
pub use storage::Database;

/// Default daily practice goal, in minutes.
pub const DEFAULT_DAILY_GOAL_MINUTES: u32 = 10;
