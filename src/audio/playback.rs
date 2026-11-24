//! Playback of the stored WAV takes.
//!
//! Only the take being listened to is decoded, and the output callback reads
//! from that buffer without copying it. Play, pause, seek and progress all go
//! through a shared atomic position so the UI never waits on the audio thread.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

use recoder_core::{Error, Result};

/// Converts a mono `f32` sample to the device sample type.
trait OutSample: cpal::SizedSample {
    fn from_f32(value: f32) -> Self;
}

impl OutSample for f32 {
    fn from_f32(value: f32) -> Self {
        value
    }
}

impl OutSample for i16 {
    fn from_f32(value: f32) -> Self {
        (value.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }
}

impl OutSample for u16 {
    fn from_f32(value: f32) -> Self {
        ((value.clamp(-1.0, 1.0) * 32_767.0) + 32_768.0) as u16
    }
}

struct Shared {
    source: Mutex<Option<Arc<Vec<f32>>>>,
    position: AtomicU64,
    src_rate: AtomicU32,
    playing: AtomicBool,
}

pub struct Player {
    shared: Arc<Shared>,
    stream: Option<cpal::Stream>,
    source_rate: u32,
    source_frames: usize,
    path: Option<PathBuf>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                source: Mutex::new(None),
                position: AtomicU64::new(0f64.to_bits()),
                src_rate: AtomicU32::new(48_000),
                playing: AtomicBool::new(false),
            }),
            stream: None,
            source_rate: 48_000,
            source_frames: 0,
            path: None,
        }
    }

    /// Decodes one file and rewinds the player. Returns the duration in ms.
    pub fn load(&mut self, path: &Path) -> Result<i64> {
        let (samples, sample_rate) = decode(path)?;
        let duration_ms = (samples.len() as f64 * 1000.0 / sample_rate as f64).round() as i64;

        self.ensure_stream()?;

        self.shared.src_rate.store(sample_rate, Ordering::SeqCst);
        self.shared.playing.store(false, Ordering::SeqCst);
        self.shared
            .position
            .store(0f64.to_bits(), Ordering::SeqCst);
        *self
            .shared
            .source
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::new(samples));

        self.source_rate = sample_rate;
        self.source_frames = self.buffered_frames();
        self.path = Some(path.to_path_buf());
        Ok(duration_ms)
    }

    pub fn play(&mut self) {
        if self.source_frames == 0 {
            return;
        }
        let position = self.position_frames();
        if position >= self.source_frames as f64 - 1.0 {
            self.seek(0);
        }
        if let Some(stream) = self.stream.as_ref() {
            let _ = stream.play();
        }
        self.shared.playing.store(true, Ordering::SeqCst);
    }

    pub fn pause(&mut self) {
        self.shared.playing.store(false, Ordering::SeqCst);
        if let Some(stream) = self.stream.as_ref() {
            let _ = stream.pause();
        }
    }

    pub fn stop(&mut self) {
        self.shared.playing.store(false, Ordering::SeqCst);
        self.shared.position.store(0f64.to_bits(), Ordering::SeqCst);
        if let Some(stream) = self.stream.as_ref() {
            let _ = stream.pause();
        }
    }

    pub fn seek(&mut self, position_ms: u64) {
        if self.source_rate == 0 {
            return;
        }
        let frames = position_ms as f64 / 1000.0 * self.source_rate as f64;
        let frames = frames.min(self.source_frames as f64);
        self.shared
            .position
            .store(frames.to_bits(), Ordering::SeqCst);
    }

    pub fn position_ms(&self) -> u64 {
        if self.source_rate == 0 {
            return 0;
        }
        (self.position_frames() / self.source_rate as f64 * 1000.0) as u64
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::SeqCst)
    }

    fn position_frames(&self) -> f64 {
        f64::from_bits(self.shared.position.load(Ordering::SeqCst))
    }

    fn buffered_frames(&self) -> usize {
        self.shared
            .source
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|samples| samples.len())
            .unwrap_or(0)
    }

    fn ensure_stream(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| Error::Audio("no audio output available".into()))?;
        let supported = device
            .default_output_config()
            .map_err(|err| Error::Audio(format!("output unavailable: {err}")))?;
        let sample_format = supported.sample_format();
        let device_channels = supported.channels().max(1) as usize;
        let device_rate = supported.sample_rate();
        let config = supported.into();

        let shared = self.shared.clone();
        let on_error = |err: cpal::Error| log::error!("playback stream error: {err}");

        self.stream = Some(match sample_format {
            SampleFormat::F32 => build_output::<f32>(
                &device,
                config,
                device_channels,
                device_rate,
                shared,
                on_error,
            ),
            SampleFormat::I16 => build_output::<i16>(
                &device,
                config,
                device_channels,
                device_rate,
                shared,
                on_error,
            ),
            SampleFormat::U16 => build_output::<u16>(
                &device,
                config,
                device_channels,
                device_rate,
                shared,
                on_error,
            ),
            other => {
                return Err(Error::Audio(format!(
                    "unsupported output format: {other:?}"
                )))
            }
        }
        .map_err(|err| Error::Audio(format!("cannot open output: {err}")))?);

        Ok(())
    }
}

fn build_output<T: OutSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    device_rate: u32,
    shared: Arc<Shared>,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> std::result::Result<cpal::Stream, String> {
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                fill(&shared, channels, device_rate, output);
            },
            on_error,
            None,
        )
        .map_err(|err| err.to_string())
}

fn fill<T: OutSample>(shared: &Shared, channels: usize, device_rate: u32, output: &mut [T]) {
    let Ok(source) = shared.source.try_lock() else {
        for slot in output.iter_mut() {
            *slot = T::from_f32(0.0);
        }
        return;
    };
    let Some(samples) = source.as_ref() else {
        for slot in output.iter_mut() {
            *slot = T::from_f32(0.0);
        }
        return;
    };

    let step = shared.src_rate.load(Ordering::Relaxed) as f64 / device_rate.max(1) as f64;
    let playing = shared.playing.load(Ordering::Relaxed);
    let total = samples.len();
    let mut position = f64::from_bits(shared.position.load(Ordering::Relaxed));

    for frame in output.chunks_mut(channels) {
        let value = if playing && (position as usize) < total {
            let index = position as usize;
            let fraction = (position - index as f64) as f32;
            let current = samples[index];
            let next = samples.get(index + 1).copied().unwrap_or(current);
            current + (next - current) * fraction
        } else {
            0.0
        };
        for slot in frame.iter_mut() {
            *slot = T::from_f32(value);
        }
        if playing {
            position += step;
        }
    }

    if playing {
        if position as usize >= total {
            shared.playing.store(false, Ordering::Relaxed);
            shared
                .position
                .store((total as f64).to_bits(), Ordering::Relaxed);
        } else {
            shared.position.store(position.to_bits(), Ordering::Relaxed);
        }
    }
}

/// Reads a WAV file into mono `f32` samples without keeping the file open.
fn decode(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|err| Error::Audio(format!("cannot read audio file: {err}")))?;
    let spec = reader.spec();
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err(Error::Audio("invalid audio file".into()));
    }
    let channels = spec.channels as usize;
    let mut mono: Vec<f32> = Vec::new();
    let mut accumulator = 0.0f32;
    let mut in_frame = 0usize;

    let mut push = |value: f32, mono: &mut Vec<f32>| {
        accumulator += value;
        in_frame += 1;
        if in_frame == channels {
            mono.push(accumulator / channels as f32);
            accumulator = 0.0;
            in_frame = 0;
        }
    };

    match spec.sample_format {
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let sample =
                    sample.map_err(|err| Error::Audio(format!("cannot decode audio: {err}")))?;
                push(sample, &mut mono);
            }
        }
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1i64 << (spec.bits_per_sample.saturating_sub(1).min(31))) as f32;
            for sample in reader.samples::<i32>() {
                let sample =
                    sample.map_err(|err| Error::Audio(format!("cannot decode audio: {err}")))?;
                push(sample as f32 * scale, &mut mono);
            }
        }
    }

    if mono.is_empty() {
        return Err(Error::Audio("empty audio file".into()));
    }
    Ok((mono, spec.sample_rate))
}
