//! Non-Android stubs. Recoder is an Android app: audio capture and playback
//! rely on AAudio through cpal, so on other targets the engine reports a clear
//! error instead of faking a recording.

use std::path::Path;

use recoder_core::{Error, Result};

use super::Take;

pub struct Recorder;

impl Recorder {
    pub fn start(_directory: &Path) -> Result<Self> {
        Err(Error::Audio(
            "audio capture is available on Android only".into(),
        ))
    }

    pub fn level(&self) -> f32 {
        0.0
    }

    pub fn elapsed_ms(&self) -> u64 {
        0
    }

    pub fn finish(self) -> Result<Take> {
        Err(Error::Audio("no recording in progress".into()))
    }
}

pub struct Player;

impl Player {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&mut self, _path: &Path) -> Result<i64> {
        Err(Error::Audio(
            "audio playback is available on Android only".into(),
        ))
    }

    pub fn play(&mut self) {}

    pub fn pause(&mut self) {}

    pub fn stop(&mut self) {}

    pub fn seek(&mut self, _position_ms: u64) {}

    pub fn position_ms(&self) -> u64 {
        0
    }

    pub fn is_playing(&self) -> bool {
        false
    }
}
