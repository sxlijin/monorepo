use anyhow::{Context, Result};
use atomic_float::{AtomicF32, AtomicF64};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, warn};

pub const MIN_PLAYBACK_SPEED: f32 = 0.2;
pub const MAX_PLAYBACK_SPEED: f32 = 3.0;

use super::wav_loader::{MappedAudioFile, WavLoader};

#[derive(Debug, Clone)]
pub enum MultiAudioCommand {
    Play,
    Pause,
    Stop,
    Seek(f64),
    LoadFiles(Vec<String>), // Load multiple audio files
    SetVolume(usize, f32),  // Set volume for specific file index
    ToggleMute(usize),      // Toggle mute state for specific file
    SoloTrack(usize),       // Solo a track (zero all others)
    SetMasterVolume(f32),
    SetPlaybackSpeed(f32),
}

#[derive(Debug, Clone)]
pub struct MultiAudioState {
    pub is_playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub master_volume: f32,
    pub playback_speed: f32,
    pub file_count: usize,
    pub file_volumes: Vec<f32>,
    pub file_mutes: Vec<bool>, // Derived from volume == 0.0
    pub file_names: Vec<String>,
}

impl Default for MultiAudioState {
    fn default() -> Self {
        Self {
            is_playing: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            master_volume: 1.0,
            playback_speed: 1.0,
            file_count: 0,
            file_volumes: Vec::new(),
            file_mutes: Vec::new(),
            file_names: Vec::new(),
        }
    }
}

struct MultiAudioEngineInner {
    audio_files: Vec<MappedAudioFile>,
    // Song-time playback position, expressed in device-rate frames so that
    // position_seconds = position_frames / sample_rate. Stored as f64 because
    // non-integer playback speeds advance by fractional amounts per buffer.
    position_frames: AtomicF64,
    is_playing: AtomicBool,
    master_volume: AtomicF32,
    playback_speed: AtomicF32,
    file_volumes: Vec<AtomicF32>,
    sample_rate: u32,
}

impl MultiAudioEngineInner {
    fn new(sample_rate: u32) -> Self {
        Self {
            audio_files: Vec::new(),
            position_frames: AtomicF64::new(0.0),
            is_playing: AtomicBool::new(false),
            master_volume: AtomicF32::new(1.0),
            playback_speed: AtomicF32::new(1.0),
            file_volumes: Vec::new(),
            sample_rate,
        }
    }

    fn load_files(&mut self, file_paths: Vec<String>) -> Result<()> {
        info!("Loading {} audio files", file_paths.len());

        let mut files = Vec::new();
        for path in &file_paths {
            match WavLoader::load_file_mapped(path) {
                Ok(file) => {
                    debug!(
                        "Memory-mapped file: {} ({} samples)",
                        path, file.sample_count
                    );
                    files.push(file);
                }
                Err(e) => {
                    warn!("Failed to memory-map file {}: {}", path, e);
                    return Err(e);
                }
            }
        }

        let file_count = files.len();
        self.file_volumes = (0..file_count).map(|_| AtomicF32::new(1.0)).collect();
        self.audio_files = files;
        self.position_frames.store(0.0, Ordering::SeqCst);

        info!("Successfully loaded {} files", file_count);
        Ok(())
    }

    fn get_max_duration_samples(&self) -> usize {
        let device_rate_f = self.sample_rate as f64;
        self.audio_files
            .iter()
            .map(|file| {
                let file_frames = file.sample_count / file.spec.channels as usize;
                let file_rate_f = file.spec.sample_rate as f64;
                (file_frames as f64 * device_rate_f / file_rate_f).ceil() as usize
            })
            .max()
            .unwrap_or(0)
    }

    fn mix_audio_callback(&self, output: &mut [f32]) {
        let position_frames = self.position_frames.load(Ordering::SeqCst);
        let is_playing = self.is_playing.load(Ordering::SeqCst);

        if !is_playing {
            // Fill with silence
            output.fill(0.0);
            return;
        }

        // Load atomic values without any locks for maximum performance
        let master_vol = self.master_volume.load(Ordering::SeqCst);
        let speed = self
            .playback_speed
            .load(Ordering::SeqCst)
            .clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED) as f64;
        let device_rate_f = self.sample_rate as f64;

        // Fill output buffer with mixed audio
        for (i, output_sample) in output.chunks_exact_mut(2).enumerate() {
            // Song-time position in device-rate frames. Speed > 1 walks through
            // the source faster (and pitches up); speed < 1 the opposite.
            let song_frame = position_frames + (i as f64) * speed;
            let mut mixed_left = 0.0f32;
            let mut mixed_right = 0.0f32;

            // Mix all audio files using volume as single source of truth
            for (file_idx, audio_file) in self.audio_files.iter().enumerate() {
                let file_vol = self
                    .file_volumes
                    .get(file_idx)
                    .map(|atomic_vol| atomic_vol.load(Ordering::SeqCst))
                    .unwrap_or(0.0);

                // Skip if volume is zero (handles mute/solo states)
                if file_vol <= 0.0 {
                    continue;
                }

                // Resample on the fly with linear interpolation between adjacent
                // source frames.
                let channels = audio_file.spec.channels as usize;
                let total_frames = audio_file.sample_count / channels;
                let file_rate_f = audio_file.spec.sample_rate as f64;
                let file_frame_pos = song_frame * file_rate_f / device_rate_f;
                if file_frame_pos < 0.0 {
                    continue;
                }
                let file_frame_lo = file_frame_pos.floor() as usize;
                if file_frame_lo + 1 >= total_frames {
                    continue; // Past end of this file
                }
                let frac = (file_frame_pos - file_frame_lo as f64) as f32;
                let file_frame_hi = file_frame_lo + 1;

                if channels == 1 {
                    let s_lo = audio_file.get_sample(file_frame_lo);
                    let s_hi = audio_file.get_sample(file_frame_hi);
                    let s = (s_lo * (1.0 - frac) + s_hi * frac) * file_vol;
                    mixed_left += s;
                    mixed_right += s;
                } else if channels == 2 {
                    let lo_idx = file_frame_lo * 2;
                    let hi_idx = file_frame_hi * 2;
                    let l_lo = audio_file.get_sample(lo_idx);
                    let l_hi = audio_file.get_sample(hi_idx);
                    let r_lo = audio_file.get_sample(lo_idx + 1);
                    let r_hi = audio_file.get_sample(hi_idx + 1);
                    let l = (l_lo * (1.0 - frac) + l_hi * frac) * file_vol;
                    let r = (r_lo * (1.0 - frac) + r_hi * frac) * file_vol;
                    mixed_left += l;
                    mixed_right += r;
                }
            }

            // Apply master volume and write to output
            output_sample[0] = (mixed_left * master_vol).clamp(-1.0, 1.0);
            output_sample[1] = (mixed_right * master_vol).clamp(-1.0, 1.0);
        }

        // Advance song-time position by `frames_processed * speed`
        let frames_processed = output.len() / 2;
        let new_position = position_frames + (frames_processed as f64) * speed;
        self.position_frames.store(new_position, Ordering::SeqCst);

        // Check if we've reached the end
        let max_duration = self.get_max_duration_samples();
        if new_position >= max_duration as f64 {
            self.is_playing.store(false, Ordering::SeqCst);
        }
    }

    fn snapshot_state(&self) -> MultiAudioState {
        let position_frames = self.position_frames.load(Ordering::SeqCst);
        let position_seconds = position_frames / self.sample_rate as f64;

        let max_duration_samples = self.get_max_duration_samples();
        let duration_seconds = max_duration_samples as f64 / self.sample_rate as f64;

        let master_volume = self.master_volume.load(Ordering::SeqCst);
        let playback_speed = self.playback_speed.load(Ordering::SeqCst);
        let file_volumes: Vec<f32> = self
            .file_volumes
            .iter()
            .map(|atomic_vol| atomic_vol.load(Ordering::SeqCst))
            .collect();

        let file_mutes = file_volumes.iter().map(|&vol| vol <= 0.0).collect();

        let file_names = self
            .audio_files
            .iter()
            .map(|file| file.path.clone())
            .collect();

        MultiAudioState {
            is_playing: self.is_playing.load(Ordering::SeqCst),
            position_seconds,
            duration_seconds,
            master_volume,
            playback_speed,
            file_count: self.audio_files.len(),
            file_volumes,
            file_mutes,
            file_names,
        }
    }
}

pub struct MultiAudioEngine {
    inner: Arc<MultiAudioEngineInner>,
    _stream: Stream,
    command_sender: std::sync::mpsc::Sender<MultiAudioCommand>,
    device_name: String,
}

impl MultiAudioEngine {
    pub fn new(device: Device) -> Result<Self> {
        Self::new_with_files(device, Vec::<String>::new())
    }

    pub fn new_with_files(device: Device, file_paths: Vec<String>) -> Result<Self> {
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());
        info!("Initializing MultiAudioEngine with device: {}", device_name);

        let config = device
            .default_output_config()
            .context("Failed to get default output config")?;

        let sample_format = config.sample_format();
        let config: StreamConfig = config.into();

        info!(
            "Audio config: {} Hz, {} channels, {:?}",
            config.sample_rate.0, config.channels, sample_format
        );

        // Create command channel
        let (command_sender, command_receiver) = std::sync::mpsc::channel::<MultiAudioCommand>();

        // Create engine inner state
        let mut inner_state = MultiAudioEngineInner::new(config.sample_rate.0);

        // Load files if provided
        if !file_paths.is_empty() {
            inner_state.load_files(file_paths.clone())?;

            for (i, file) in inner_state.audio_files.iter().enumerate() {
                info!(
                    "File {}: {}Hz, {} channels, {} samples",
                    i, file.spec.sample_rate, file.spec.channels, file.sample_count
                );
            }
            info!(
                "Audio device: {}Hz, {} channels",
                config.sample_rate.0, config.channels
            );
        }

        let inner = Arc::new(inner_state);

        // Clone for the audio callback
        let callback_inner = inner.clone();

        // Build the audio stream
        let stream = match sample_format {
            SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    callback_inner.mix_audio_callback(data);
                },
                |err| error!("Audio stream error: {}", err),
                None,
            )?,
            SampleFormat::I16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        // Convert to f32, process, then convert back
                        let mut float_data: Vec<f32> =
                            data.iter().map(|&sample| sample.to_sample()).collect();

                        callback_inner.mix_audio_callback(&mut float_data);

                        for (i, &sample) in float_data.iter().enumerate() {
                            data[i] = sample.to_sample();
                        }
                    },
                    |err| error!("Audio stream error: {}", err),
                    None,
                )?
            }
            SampleFormat::U16 => {
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        // Convert to f32, process, then convert back
                        let mut float_data: Vec<f32> =
                            data.iter().map(|&sample| sample.to_sample()).collect();

                        callback_inner.mix_audio_callback(&mut float_data);

                        for (i, &sample) in float_data.iter().enumerate() {
                            data[i] = sample.to_sample();
                        }
                    },
                    |err| error!("Audio stream error: {}", err),
                    None,
                )?
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported sample format: {:?}",
                    sample_format
                ));
            }
        };

        stream.play().context("Failed to start audio stream")?;

        // Spawn command processing thread
        let command_inner = inner.clone();
        thread::spawn(move || {
            for command in command_receiver {
                Self::process_command(&command_inner, command);
            }
        });

        Ok(Self {
            inner,
            _stream: stream,
            command_sender,
            device_name,
        })
    }

    fn process_command(inner: &MultiAudioEngineInner, command: MultiAudioCommand) {
        match command {
            MultiAudioCommand::Play => {
                debug!("Processing play command");
                inner.is_playing.store(true, Ordering::SeqCst);
            }
            MultiAudioCommand::Pause => {
                debug!("Processing pause command");
                inner.is_playing.store(false, Ordering::SeqCst);
            }
            MultiAudioCommand::Stop => {
                debug!("Processing stop command");
                inner.is_playing.store(false, Ordering::SeqCst);
                inner.position_frames.store(0.0, Ordering::SeqCst);
            }
            MultiAudioCommand::Seek(position_seconds) => {
                debug!("Processing seek command to {}s", position_seconds);
                let position_frames = position_seconds.max(0.0) * inner.sample_rate as f64;
                inner.position_frames.store(position_frames, Ordering::SeqCst);
            }
            MultiAudioCommand::LoadFiles(_file_paths) => {
                debug!("Processing load files command");
                // Note: File loading needs to be handled differently due to borrowing rules
                warn!("LoadFiles command received but cannot modify inner state from callback");
            }
            MultiAudioCommand::SetVolume(file_idx, volume) => {
                debug!("Setting volume for file {} to {}", file_idx, volume);
                if let Some(atomic_volume) = inner.file_volumes.get(file_idx) {
                    atomic_volume.store(volume.clamp(0.0, 2.0), Ordering::SeqCst);
                }
            }
            MultiAudioCommand::ToggleMute(file_idx) => {
                debug!("Toggling mute for file {}", file_idx);
                Self::toggle_mute(inner, file_idx);
            }
            MultiAudioCommand::SoloTrack(file_idx) => {
                debug!("Soloing track {}", file_idx);
                Self::solo_track(inner, file_idx);
            }
            MultiAudioCommand::SetMasterVolume(volume) => {
                debug!("Setting master volume to {}", volume);
                inner
                    .master_volume
                    .store(volume.clamp(0.0, 2.0), Ordering::SeqCst);
            }
            MultiAudioCommand::SetPlaybackSpeed(speed) => {
                let clamped = speed.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED);
                debug!("Setting playback speed to {}", clamped);
                inner.playback_speed.store(clamped, Ordering::SeqCst);
            }
        }
    }

    fn toggle_mute(inner: &MultiAudioEngineInner, file_idx: usize) {
        if file_idx >= inner.file_volumes.len() {
            return;
        }

        let current_volume = inner.file_volumes[file_idx].load(Ordering::SeqCst);

        if current_volume > 0.0 {
            // Currently audible, mute it (set to 0.0)
            inner.file_volumes[file_idx].store(0.0, Ordering::SeqCst);
        } else {
            // Currently muted, unmute it (set to 1.0)
            inner.file_volumes[file_idx].store(1.0, Ordering::SeqCst);
        }
    }

    fn solo_track(inner: &MultiAudioEngineInner, file_idx: usize) {
        if file_idx >= inner.file_volumes.len() {
            return;
        }

        // Get current volume of the track to solo
        let current_volume = inner.file_volumes[file_idx].load(Ordering::SeqCst);

        // If the soloed track is effectively muted, bring it back to unity so it becomes audible again.
        if current_volume <= 0.0 {
            inner.file_volumes[file_idx].store(1.0, Ordering::SeqCst);
        }

        // Zero out all other tracks
        for (idx, atomic_vol) in inner.file_volumes.iter().enumerate() {
            if idx != file_idx {
                atomic_vol.store(0.0, Ordering::SeqCst);
            }
        }

        // Keep the solo track at its current volume (no change needed)
    }

    pub fn send_command(&self, command: MultiAudioCommand) -> Result<()> {
        self.command_sender
            .send(command)
            .context("Failed to send command to audio engine")
    }

    pub fn get_state(&self) -> MultiAudioState {
        self.inner.snapshot_state()
    }

    pub fn get_device_name(&self) -> &str {
        &self.device_name
    }

    pub fn state_supplier(&self) -> Arc<dyn Fn() -> MultiAudioState + Send + Sync + 'static> {
        let inner = Arc::clone(&self.inner);
        Arc::new(move || inner.snapshot_state())
    }
}
