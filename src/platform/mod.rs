//! Thin Rust bridge to the Android platform.

use std::path::PathBuf;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::{app_files_dir, has_microphone_permission, is_permission_blocked, request_microphone_permission};

#[cfg(not(target_os = "android"))]
mod fallback;
#[cfg(not(target_os = "android"))]
pub use fallback::{app_files_dir, has_microphone_permission, is_permission_blocked, request_microphone_permission};

/// Directory owned by the app, used for the database and the audio files.
pub fn storage_root() -> Option<PathBuf> {
    app_files_dir().map(|dir| dir.join("recoder"))
}
