//! Development fallback. Recoder ships for Android only: these stubs keep the
//! crate checkable on a workstation without pretending audio works there.

use std::path::PathBuf;
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn app_files_dir() -> Option<PathBuf> {
    Some(ROOT.get_or_init(|| std::env::temp_dir().join("recoder-dev")).clone())
}

pub fn has_microphone_permission() -> bool {
    true
}

pub fn request_microphone_permission() {}

pub fn is_permission_blocked() -> bool {
    false
}
