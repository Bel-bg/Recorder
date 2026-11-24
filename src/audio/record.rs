//! Microphone capture through cpal (AAudio on Android).
//!
//! Mono samples are pushed to a writer thread that appends them to a WAV file
//! as they arrive, so memory usage stays flat during long sessions. The most
//! recent RMS amplitude is exposed for the live waveform.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use recoder_core::{Error, Result};

use super::Take;

struct Shared {
    level: AtomicU32,
    frames: AtomicU64,
    sample_rate: u32,
    failed: AtomicBool,
}

pub struct Recorder {
    stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    sender: Option<SyncSender<Vec<f32>>>,
    writer: Option<JoinHandle<()>>,
    id: String,
    path: PathBuf,
}

impl Recorder {
    pub fn start(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory)?;

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| Error::Audio("no microphone available".into()))?;
        let supported = device
            .default_input_config()
            .map_err(|err| Error::Audio(format!("microphone unavailable: {err}")))?;

        let sample_format = supported.sample_format();
        let sample_rate = supported.sample_rate();
        let channels = supported.channels().max(1) as usize;
        let config: StreamConfig = supported.into();

        let id = uuid::Uuid::new_v4().to_string();
        let path = directory.join(format!("{id}.wav"));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let (sender, receiver) = sync_channel::<Vec<f32>>(256);
        let writer_path = path.clone();
        let writer = std::thread::Builder::new()
            .name("recoder-wav".into())
            .spawn(move || {
                let mut wav = match hound::WavWriter::create(&writer_path, spec) {
                    Ok(writer) => writer,
                    Err(err) => {
                        log::error!("cannot create wav file: {err}");
                        return;
                    }
                };
                while let Ok(chunk) = receiver.recv() {
                    for sample in chunk {
                        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        if wav.write_sample(value).is_err() {
                            break;
                        }
                    }
                }
                if let Err(err) = wav.finalize() {
                    log::error!("cannot finalize wav file: {err}");
                }
            })
            .map_err(|err| Error::Audio(err.to_string()))?;

        let shared = Arc::new(Shared {
            level: AtomicU32::new(0),
            frames: AtomicU64::new(0),
            sample_rate,
            failed: AtomicBool::new(false),
        });

        let error_shared = shared.clone();
        let on_error = move |err: cpal::Error| {
            log::error!("microphone stream error: {err}");
            error_shared.failed.store(true, Ordering::SeqCst);
        };

        let stream = match sample_format {
            SampleFormat::F32 => build_stream(
                &device,
                config.clone(),
                channels,
                shared.clone(),
                sender.clone(),
                |value: &f32| *value,
                on_error,
            ),
            SampleFormat::I16 => build_stream(
                &device,
                config.clone(),
                channels,
                shared.clone(),
                sender.clone(),
                |value: &i16| *value as f32 / 32_768.0,
                on_error,
            ),
            SampleFormat::U16 => build_stream(
                &device,
                config.clone(),
                channels,
                shared.clone(),
                sender.clone(),
                |value: &u16| (*value as f32 - 32_768.0) / 32_768.0,
                on_error,
            ),
            other => Err(format!("unsupported microphone format: {other:?}").into()),
        }
        .map_err(|err| Error::Audio(format!("cannot open microphone: {err}")))?;

        stream
            .play()
            .map_err(|err| Error::Audio(format!("cannot start microphone: {err}")))?;

        Ok(Self {
            stream: Some(stream),
            shared,
            sender: Some(sender),
            writer: Some(writer),
            id,
            path,
        })
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.shared.level.load(Ordering::Relaxed))
    }

    pub fn elapsed_ms(&self) -> u64 {
        let frames = self.shared.frames.load(Ordering::Relaxed) as f64;
        (frames * 1000.0 / self.shared.sample_rate as f64) as u64
    }

    pub fn finish(mut self) -> Result<Take> {
        let duration_ms = self.shutdown();
        if self.shared.failed.load(Ordering::SeqCst) {
            // The system can take the microphone away (call, background). Keep
            // what was actually captured instead of throwing the take away.
            log::warn!("microphone stream ended early, keeping captured audio");
        }
        Ok(Take {
            id: self.id.clone(),
            path: self.path.clone(),
            duration_ms,
        })
    }

    /// Stops the stream, closes the writer and reports the captured duration.
    fn shutdown(&mut self) -> i64 {
        self.stream.take();
        self.sender.take();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        let frames = self.shared.frames.load(Ordering::SeqCst) as f64;
        (frames * 1000.0 / self.shared.sample_rate as f64).round() as i64
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    channels: usize,
    shared: Arc<Shared>,
    sender: SyncSender<Vec<f32>>,
    convert: fn(&T) -> f32,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> std::result::Result<cpal::Stream, String>
where
    T: cpal::SizedSample + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut mono = Vec::with_capacity(data.len() / channels + 1);
                let mut energy = 0.0f32;
                for frame in data.chunks(channels) {
                    let mut sum = 0.0f32;
                    for sample in frame {
                        sum += convert(sample);
                    }
                    let value = sum / frame.len().max(1) as f32;
                    energy += value * value;
                    mono.push(value);
                }
                if mono.is_empty() {
                    return;
                }
                let rms = (energy / mono.len() as f32).sqrt();
                shared.level.store(rms.to_bits(), Ordering::Relaxed);
                shared
                    .frames
                    .fetch_add(mono.len() as u64, Ordering::Relaxed);
                // A full channel means the writer is behind: dropping the chunk
                // keeps the audio thread responsive instead of blocking it.
                let _ = sender.try_send(mono);
            },
            on_error,
            None,
        )
        .map_err(|err| err.to_string())
}
