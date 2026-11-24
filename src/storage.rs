use std::path::PathBuf;

use recoder_core::{Error, Result};

use crate::platform;

/// Private Android storage layout: the database and the audio files live in
/// the app sandbox, so no storage permission is ever needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    pub root: PathBuf,
    pub recordings: PathBuf,
    pub database: PathBuf,
}

impl AppPaths {
    pub fn detect() -> Result<Self> {
        let root = platform::storage_root().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "application storage is not reachable",
            ))
        })?;
        let recordings = root.join("recordings");
        std::fs::create_dir_all(&recordings)?;
        Ok(Self {
            database: root.join("recoder.sqlite3"),
            recordings,
            root,
        })
    }
}
