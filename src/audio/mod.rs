//! Audio capture and playback.
//!
//! Capture streams the microphone straight to a WAV file through a writer
//! thread so long sessions never hold the whole take in memory, while a shared
//! amplitude value feeds the live waveform.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use recoder_core::{Error, Result};

#[cfg(target_os = "android")]
mod playback;
#[cfg(target_os = "android")]
mod record;
#[cfg(not(target_os = "android"))]
mod stub;

#[cfg(target_os = "android")]
use {playback::Player, record::Recorder};
#[cfg(not(target_os = "android"))]
use stub::{Player, Recorder};

/// A finished but not yet saved take.
#[derive(Clone, Debug)]
pub struct Take {
    pub id: String,
    pub path: PathBuf,
    pub duration_ms: i64,
}

pub struct AudioEngine {
    recorder: Mutex<Option<Recorder>>,
    player: Mutex<Player>,
    loaded: Mutex<Option<PathBuf>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            recorder: Mutex::new(None),
            player: Mutex::new(Player::new()),
            loaded: Mutex::new(None),
        }
    }

    pub fn start_recording(&self, directory: &Path) -> Result<()> {
        let mut recorder = self.lock_recorder();
        if recorder.is_some() {
            return Ok(());
        }
        *recorder = Some(Recorder::start(directory)?);
        Ok(())
    }

    pub fn level(&self) -> f32 {
        self.lock_recorder()
            .as_ref()
            .map(Recorder::level)
            .unwrap_or(0.0)
    }

    pub fn recorded_ms(&self) -> u64 {
        self.lock_recorder()
            .as_ref()
            .map(Recorder::elapsed_ms)
            .unwrap_or(0)
    }

    pub fn stop_recording(&self) -> Result<Take> {
        let recorder = self.lock_recorder().take();
        match recorder {
            Some(recorder) => recorder.finish(),
            None => Err(Error::Audio("no recording in progress".into())),
        }
    }

    pub fn discard(&self, take: &Take) {
        let _ = std::fs::remove_file(&take.path);
    }

    pub fn load(&self, path: &Path) -> Result<i64> {
        self.stop();
        let duration = self.lock_player().load(path)?;
        *self.lock_loaded() = Some(path.to_path_buf());
        Ok(duration)
    }

    pub fn play(&self) {
        self.lock_player().play();
    }

    pub fn pause(&self) {
        self.lock_player().pause();
    }

    pub fn stop(&self) {
        self.lock_player().stop();
    }

    pub fn seek(&self, position_ms: u64) {
        self.lock_player().seek(position_ms);
    }

    pub fn position_ms(&self) -> u64 {
        self.lock_player().position_ms()
    }

    pub fn is_playing(&self) -> bool {
        self.lock_player().is_playing()
    }

    fn lock_recorder(&self) -> std::sync::MutexGuard<'_, Option<Recorder>> {
        self.recorder
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_player(&self) -> std::sync::MutexGuard<'_, Player> {
        self.player
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_loaded(&self) -> std::sync::MutexGuard<'_, Option<PathBuf>> {
        self.loaded
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
