//! Application state and the actions driving it.
//!
//! Pages stay presentational: every mutation goes through `AppContext`, which
//! owns the database, the audio engine and the shared signals.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use dioxus::prelude::*;
use recoder_core::Recording;

use crate::audio::{AudioEngine, Take};
use crate::platform;
use crate::storage::AppPaths;

pub const BAR_COUNT: usize = 44;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Stats,
    Record,
    History,
}

impl Screen {
    fn position(self) -> i8 {
        match self {
            Screen::Stats => -1,
            Screen::Record => 0,
            Screen::History => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Capture {
    /// Nothing in progress.
    Idle,
    /// The microphone permission is explained before asking Android.
    Permission,
    /// Android is showing its permission dialog.
    Arming,
    Recording,
    /// A take is waiting to be kept or deleted.
    Review,
    /// A take was saved, the UI confirms briefly.
    Saved,
}

#[derive(Clone, Copy)]
pub struct Studio {
    pub state: Signal<Capture>,
    pub take: Signal<Option<Take>>,
    pub error: Signal<Option<String>>,
    pub elapsed_ms: Signal<u64>,
    pub bars: Signal<Vec<f32>>,
}

#[derive(Clone, Copy)]
pub struct Playback {
    pub recording_id: Signal<Option<String>>,
    pub path: Signal<Option<String>>,
    pub duration_ms: Signal<u64>,
    pub position_ms: Signal<u64>,
    pub playing: Signal<bool>,
    /// True when the bottom player panel should be visible.
    pub open: Signal<bool>,
}

#[derive(Clone)]
pub struct AppContext {
    pub paths: AppPaths,
    pub db: Arc<recoder_core::Database>,
    pub audio: Arc<AudioEngine>,
    pub studio: Studio,
    pub playback: Playback,
    pub screen: Signal<Screen>,
    pub direction: Signal<i8>,
    pub revision: Signal<u64>,
}

impl AppContext {
    // ---------------------------------------------------------------- data

    pub fn recordings(&self) -> Vec<Recording> {
        self.db.recordings().unwrap_or_else(|err| {
            log::error!("cannot read recordings: {err}");
            Vec::new()
        })
    }

    pub fn daily_goal_minutes(&self) -> u32 {
        self.db.daily_goal_minutes()
    }

    pub fn set_daily_goal_minutes(&self, minutes: u32) {
        let minutes = minutes.clamp(1, 240);
        if let Err(err) = self.db.set_daily_goal_minutes(minutes) {
            log::error!("cannot save daily goal: {err}");
        }
        let mut revision = self.revision;
        revision += 1;
    }

    fn touch(&self) {
        let mut revision = self.revision;
        revision += 1;
    }

    // ------------------------------------------------------------ navig

    pub fn go_to(&self, target: Screen) {
        let mut screen = self.screen;
        let mut direction = self.direction;
        let current = screen();
        if current == target {
            return;
        }
        direction.set(if target.position() > current.position() {
            1
        } else {
            -1
        });
        screen.set(target);
        if target != Screen::History {
            self.close_player();
        }
    }

    // --------------------------------------------------------- recording

    pub fn handle_record_button(&self) {
        match self.studio.state.cloned() {
            Capture::Recording => self.stop_recording(),
            Capture::Idle | Capture::Saved => self.start_recording(),
            Capture::Permission | Capture::Arming | Capture::Review => {}
        }
    }

    pub fn start_recording(&self) {
        let mut studio = self.studio;
        studio.error.set(None);
        if platform::has_microphone_permission() {
            self.begin_capture();
            return;
        }
        if platform::is_permission_blocked() {
            studio.error.set(Some(
                "Le microphone est bloqué. Autorisez-le dans les réglages Android de Recoder."
                    .into(),
            ));
            studio.state.set(Capture::Idle);
            return;
        }
        studio.state.set(Capture::Permission);
    }

    /// Called when the user accepts the explanation card.
    pub fn authorize_microphone(&self) {
        let mut studio = self.studio;
        studio.error.set(None);
        studio.state.set(Capture::Arming);
        platform::request_microphone_permission();

        let context = self.clone();
        spawn(async move {
            for _ in 0..250u32 {
                tokio::time::sleep(Duration::from_millis(120)).await;
                if platform::has_microphone_permission() {
                    context.begin_capture();
                    return;
                }
            }
            let mut studio = context.studio;
            studio.state.set(Capture::Idle);
            studio
                .error
                .set(Some("Le microphone n'a pas été autorisé.".into()));
        });
    }

    fn begin_capture(&self) {
        let mut studio = self.studio;
        match self.audio.start_recording(&self.paths.recordings) {
            Ok(()) => {
                studio.state.set(Capture::Recording);
                studio.elapsed_ms.set(0);
                studio.bars.set(vec![0.0; BAR_COUNT]);
            }
            Err(err) => {
                log::error!("cannot start recording: {err}");
                studio.state.set(Capture::Idle);
                studio
                    .error
                    .set(Some("Le microphone n'est pas disponible pour le moment.".into()));
            }
        }
    }

    pub fn stop_recording(&self) {
        let mut studio = self.studio;
        match self.audio.stop_recording() {
            Ok(take) if take.duration_ms < 600 => {
                self.audio.discard(&take);
                studio.state.set(Capture::Idle);
                studio.elapsed_ms.set(0);
                studio
                    .error
                    .set(Some("Prise trop courte pour être gardée.".into()));
            }
            Ok(take) => {
                studio.elapsed_ms.set(take.duration_ms.max(0) as u64);
                studio.take.set(Some(take));
                studio.state.set(Capture::Review);
            }
            Err(err) => {
                log::error!("cannot stop recording: {err}");
                studio.state.set(Capture::Idle);
                studio
                    .error
                    .set(Some("L'enregistrement s'est mal terminé.".into()));
            }
        }
    }

    pub fn save_take(&self) {
        let mut studio = self.studio;
        let Some(take) = studio.take.cloned() else {
            return;
        };
        let recording = Recording {
            id: take.id.clone(),
            path: take.path.to_string_lossy().to_string(),
            duration_ms: take.duration_ms,
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        match self.db.insert_recording(&recording) {
            Ok(()) => {
                studio.take.set(None);
                studio.state.set(Capture::Saved);
                self.touch();

                let context = self.clone();
                spawn(async move {
                    tokio::time::sleep(Duration::from_millis(1600)).await;
                    let mut studio = context.studio;
                    if studio.state.cloned() == Capture::Saved {
                        studio.state.set(Capture::Idle);
                        studio.elapsed_ms.set(0);
                    }
                });
            }
            Err(err) => {
                log::error!("cannot save take: {err}");
                studio
                    .error
                    .set(Some("La sauvegarde a échoué. Réessayez.".into()));
            }
        }
    }

    pub fn discard_take(&self) {
        let mut studio = self.studio;
        if let Some(take) = studio.take.cloned() {
            if self.playback.path.cloned() == Some(take.path.to_string_lossy().to_string()) {
                self.close_player();
            }
            self.audio.discard(&take);
        }
        studio.take.set(None);
        studio.state.set(Capture::Idle);
        studio.elapsed_ms.set(0);
    }

    pub fn delete_recording(&self, recording: &Recording) {
        match self.db.delete_recording(&recording.id) {
            Ok(path) => {
                if let Some(path) = path {
                    if let Err(err) = std::fs::remove_file(Path::new(&path)) {
                        log::warn!("cannot remove audio file {path}: {err}");
                    }
                }
                if self.playback.recording_id.cloned() == Some(recording.id.clone()) {
                    self.close_player();
                }
                self.touch();
            }
            Err(err) => log::error!("cannot delete recording: {err}"),
        }
    }

    // ----------------------------------------------------------- playback

    /// Plays the take under review, without opening the history player.
    pub fn toggle_take_playback(&self) {
        let mut playback = self.playback;
        let mut studio = self.studio;
        let Some(take) = studio.take.cloned() else {
            return;
        };
        let path = take.path.to_string_lossy().to_string();

        if playback.path.cloned() == Some(path) {
            self.toggle_playback();
            return;
        }
        match self.audio.load(&take.path) {
            Ok(duration) => {
                playback.path.set(Some(take.path.to_string_lossy().to_string()));
                playback.recording_id.set(None);
                playback.duration_ms.set(duration.max(0) as u64);
                playback.position_ms.set(0);
                playback.playing.set(true);
                self.audio.play();
            }
            Err(err) => {
                log::error!("cannot play take: {err}");
                studio
                    .error
                    .set(Some("La lecture de cette prise est impossible.".into()));
            }
        }
    }

    pub fn open_recording(&self, recording: &Recording) {
        let mut playback = self.playback;
        let mut studio = self.studio;

        if playback.recording_id.cloned() == Some(recording.id.clone()) {
            playback.open.set(true);
            self.toggle_playback();
            return;
        }
        match self.audio.load(Path::new(&recording.path)) {
            Ok(duration) => {
                playback.recording_id.set(Some(recording.id.clone()));
                playback.path.set(Some(recording.path.clone()));
                playback.duration_ms.set(duration.max(0) as u64);
                playback.position_ms.set(0);
                playback.open.set(true);
                playback.playing.set(true);
                self.audio.play();
            }
            Err(err) => {
                log::error!("cannot open recording: {err}");
                studio
                    .error
                    .set(Some("Ce fichier audio ne peut pas être lu.".into()));
            }
        }
    }

    pub fn toggle_playback(&self) {
        let mut playback = self.playback;
        if playback.playing.cloned() {
            self.audio.pause();
            playback.playing.set(false);
            return;
        }
        if playback.path.cloned().is_none() {
            return;
        }
        self.audio.play();
        playback.playing.set(true);
    }

    pub fn seek_playback(&self, position_ms: u64) {
        let mut playback = self.playback;
        self.audio.seek(position_ms);
        playback.position_ms.set(position_ms);
    }

    pub fn stop_playback(&self) {
        let mut playback = self.playback;
        self.audio.stop();
        playback.playing.set(false);
        playback.position_ms.set(0);
    }

    pub fn close_player(&self) {
        let mut playback = self.playback;
        self.audio.stop();
        playback.open.set(false);
        playback.playing.set(false);
        playback.position_ms.set(0);
        playback.recording_id.set(None);
        playback.path.set(None);
        playback.duration_ms.set(0);
    }

    // --------------------------------------------------------------- tick

    /// Refreshes the live signals. Called from a single background task.
    pub fn tick(&self) {
        let mut studio = self.studio;
        let mut playback = self.playback;

        if studio.state.cloned() == Capture::Recording {
            let level = self.audio.level();
            studio.elapsed_ms.set(self.audio.recorded_ms());
            studio.bars.with_mut(|bars| {
                if bars.len() != BAR_COUNT {
                    bars.resize(BAR_COUNT, 0.0);
                }
                bars.rotate_left(1);
                if let Some(last) = bars.last_mut() {
                    *last = level;
                }
            });
        }

        if playback.path.cloned().is_some() {
            playback.position_ms.set(self.audio.position_ms());
            playback.playing.set(self.audio.is_playing());
        }
    }
}
