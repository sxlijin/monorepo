use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, Stream, StreamConfig};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use super::{AudioFile, WavLoader};

#[derive(Debug)]
pub enum AudioCommand {
    Play,
    Pause,
    Stop,
    Seek(f64),
    LoadFile(String),
    SetVolume(f32),
}

#[derive(Debug, Clone)]
pub struct AudioEngineState {
    pub is_playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub volume: f32,
    pub current_file: Option<String>,
}

impl Default for AudioEngineState {
    fn default() -> Self {
        Self {
            is_playing: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            volume: 1.0,
            current_file: None,
        }
    }
}

pub struct AudioEngine {
    _stream: Stream,
    command_sender: mpsc::Sender<AudioCommand>,
    state: Arc<RwLock<AudioEngineState>>,
    device: Device,
}

impl AudioEngine {
    pub fn new(device: Device) -> Result<Self> {
        let config = device
            .default_output_config()
            .context("Failed to get default output config")?;

        tracing::info!("Audio config: {:?}", config);

        let (command_sender, command_receiver) = mpsc::channel::<AudioCommand>();
        let state = Arc::new(RwLock::new(AudioEngineState::default()));

        // Audio processing state
        let sample_position = Arc::new(AtomicUsize::new(0));
        let is_playing = Arc::new(AtomicBool::new(false));
        let current_audio = Arc::new(RwLock::new(None::<AudioFile>));
        let volume = Arc::new(RwLock::new(1.0f32));

        // Clone for audio thread
        let audio_sample_position = sample_position.clone();
        let audio_is_playing = is_playing.clone();
        let audio_current_audio = current_audio.clone();
        let audio_volume = volume.clone();
        let audio_state = state.clone();

        // Start command processing thread
        let cmd_sample_position = sample_position.clone();
        let cmd_is_playing = is_playing.clone();
        let cmd_current_audio = current_audio.clone();
        let cmd_volume = volume.clone();
        let cmd_state = state.clone();

        thread::spawn(move || {
            for command in command_receiver {
                match command {
                    AudioCommand::Play => {
                        tracing::info!("Audio engine: Play");
                        cmd_is_playing.store(true, Ordering::SeqCst);
                        cmd_state.write().is_playing = true;
                    }
                    AudioCommand::Pause => {
                        tracing::info!("Audio engine: Pause");
                        cmd_is_playing.store(false, Ordering::SeqCst);
                        cmd_state.write().is_playing = false;
                    }
                    AudioCommand::Stop => {
                        tracing::info!("Audio engine: Stop");
                        cmd_is_playing.store(false, Ordering::SeqCst);
                        cmd_sample_position.store(0, Ordering::SeqCst);
                        let mut state = cmd_state.write();
                        state.is_playing = false;
                        state.position_seconds = 0.0;
                    }
                    AudioCommand::Seek(position_seconds) => {
                        tracing::info!("Audio engine: Seek to {:.2}s", position_seconds);
                        if let Some(audio) = cmd_current_audio.read().as_ref() {
                            let sample_rate = audio.spec.sample_rate as f64;
                            let channels = audio.spec.channels as f64;
                            let target_sample =
                                (position_seconds * sample_rate * channels) as usize;
                            let max_sample = audio.samples.len();
                            let clamped_sample = target_sample.min(max_sample);
                            cmd_sample_position.store(clamped_sample, Ordering::SeqCst);
                            cmd_state.write().position_seconds = position_seconds;
                        }
                    }
                    AudioCommand::LoadFile(path) => {
                        tracing::info!("Audio engine: Loading file {}", path);
                        match WavLoader::load_file(&path) {
                            Ok(audio_file) => {
                                let duration = audio_file.duration_seconds;
                                *cmd_current_audio.write() = Some(audio_file);
                                cmd_sample_position.store(0, Ordering::SeqCst);
                                cmd_is_playing.store(false, Ordering::SeqCst);

                                let mut state = cmd_state.write();
                                state.current_file = Some(path);
                                state.duration_seconds = duration;
                                state.position_seconds = 0.0;
                                state.is_playing = false;

                                tracing::info!(
                                    "Successfully loaded audio file, duration: {:.2}s",
                                    duration
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to load audio file: {}", e);
                            }
                        }
                    }
                    AudioCommand::SetVolume(vol) => {
                        tracing::info!("Audio engine: Set volume to {:.2}", vol);
                        *cmd_volume.write() = vol.clamp(0.0, 2.0);
                        cmd_state.write().volume = vol.clamp(0.0, 2.0);
                    }
                }
            }
        });

        // Create audio stream
        let stream = match config.sample_format() {
            SampleFormat::F32 => Self::create_stream::<f32>(
                &device,
                &config.into(),
                audio_sample_position,
                audio_is_playing,
                audio_current_audio,
                audio_volume,
                audio_state,
            )?,
            SampleFormat::I16 => Self::create_stream::<i16>(
                &device,
                &config.into(),
                audio_sample_position,
                audio_is_playing,
                audio_current_audio,
                audio_volume,
                audio_state,
            )?,
            SampleFormat::U16 => Self::create_stream::<u16>(
                &device,
                &config.into(),
                audio_sample_position,
                audio_is_playing,
                audio_current_audio,
                audio_volume,
                audio_state,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play().context("Failed to start audio stream")?;

        Ok(AudioEngine {
            _stream: stream,
            command_sender,
            state,
            device,
        })
    }

    fn create_stream<T>(
        device: &Device,
        config: &StreamConfig,
        sample_position: Arc<AtomicUsize>,
        is_playing: Arc<AtomicBool>,
        current_audio: Arc<RwLock<Option<AudioFile>>>,
        volume: Arc<RwLock<f32>>,
        state: Arc<RwLock<AudioEngineState>>,
    ) -> Result<Stream>
    where
        T: Sample + cpal::SizedSample + Send + 'static + cpal::FromSample<f32>,
        f32: cpal::FromSample<T>,
    {
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0 as f64;

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                if !is_playing.load(Ordering::SeqCst) {
                    // Fill with silence when not playing
                    for sample in data.iter_mut() {
                        *sample = T::EQUILIBRIUM;
                    }
                    return;
                }

                let audio_guard = current_audio.read();
                let audio_file = match audio_guard.as_ref() {
                    Some(audio) => audio,
                    None => {
                        // No audio loaded, fill with silence
                        for sample in data.iter_mut() {
                            *sample = T::EQUILIBRIUM;
                        }
                        return;
                    }
                };

                let vol = *volume.read();
                let current_pos = sample_position.load(Ordering::SeqCst);
                let audio_samples = &audio_file.samples;

                for (i, sample) in data.iter_mut().enumerate() {
                    let audio_index = current_pos + i;

                    let audio_sample = if audio_index < audio_samples.len() {
                        audio_samples[audio_index] * vol
                    } else {
                        // End of audio, stop playback
                        is_playing.store(false, Ordering::SeqCst);
                        state.write().is_playing = false;
                        0.0
                    };

                    *sample = cpal::Sample::from_sample(audio_sample);
                }

                // Update position
                let new_pos = current_pos + data.len();
                sample_position.store(new_pos, Ordering::SeqCst);

                // Update state with current position
                let position_seconds = new_pos as f64 / (sample_rate * channels as f64);
                state.write().position_seconds = position_seconds;
            },
            move |err| {
                tracing::error!("Audio stream error: {}", err);
            },
            None,
        )?;

        Ok(stream)
    }

    pub fn send_command(&self, command: AudioCommand) -> Result<()> {
        self.command_sender
            .send(command)
            .context("Failed to send audio command")
    }

    pub fn get_state(&self) -> AudioEngineState {
        self.state.read().clone()
    }

    pub fn get_device_name(&self) -> String {
        self.device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string())
    }
}
