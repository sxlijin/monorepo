// use crate::analysis::{WaveformAnalyzer, WaveformData, WaveformPeak};
use crate::analysis::{self, RawWaveformData, SeparationResult};
use crate::audio::wav_loader::WavLoader;
use crate::audio::{
    extract_metadata, extract_metadata_from_first_file, DeviceManager, MultiAudioCommand,
    MultiAudioEngine, MultiAudioState, SongMetadata, StemDiscovery, MAX_PLAYBACK_SPEED,
    MIN_PLAYBACK_SPEED,
};
use crate::constants::StemType;
use crate::waveform_registry::WAVEFORM_REGISTRY;
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use qmetaobject::{queued_callback, QPointer, *};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use url::Url;

#[derive(Clone)]
struct StemmedFile {
    stem: StemType,
    path: String,
    exists: bool,
}

struct DownloadResult {
    audio_path: String,
}

struct StateUpdateWorker {
    stop_flag: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl StateUpdateWorker {
    fn new(
        bridge: &MultiBridge,
        state_supplier: Arc<dyn Fn() -> MultiAudioState + Send + Sync + 'static>,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);

        let bridge_ptr: QPointer<MultiBridge> = QPointer::from(bridge);
        let state_callback = queued_callback(move |state: MultiAudioState| {
            if let Some(pinned) = bridge_ptr.as_pinned() {
                let mut bridge_ref = pinned.borrow_mut();
                bridge_ref.apply_state_snapshot(state);
            }
        });

        let supplier = Arc::clone(&state_supplier);

        let handle = thread::spawn(move || {
            let interval = Duration::from_micros(2_000); // ~500 FPS target
            while !stop_flag_clone.load(Ordering::SeqCst) {
                let state = (supplier.as_ref())();
                state_callback(state);
                thread::sleep(interval);
            }
        });

        Self {
            stop_flag,
            handle: Some(handle),
        }
    }
}

impl Drop for StateUpdateWorker {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadingState {
    Idle {
        file_count: usize,
    },
    Downloading {
        file_count: usize,
        stage_message: String,
    },
    SeparatingStems {
        file_count: usize,
        stage_message: String,
        progress: f64, // 0.0 to 1.0
        waveforms_completed: usize,
        waveforms_total: usize,
    },
    LoadingAudio {
        file_count: usize,
        stage_message: String,
        progress: f64, // 0.0 to 1.0
    },
    GeneratingWaveforms {
        file_count: usize,
        stage_message: String,
        progress: f64, // 0.0 to 1.0
        waveforms_completed: usize,
        waveforms_total: usize,
    },
    Complete {
        file_count: usize,
        all_waveforms_ready: bool,
    },
    Failed {
        error_message: String,
        file_count: usize,
    },
}

impl LoadingState {
    pub fn as_string(&self) -> String {
        match self {
            LoadingState::Idle { .. } => "Idle".to_string(),
            LoadingState::Downloading { .. } => "Downloading".to_string(),
            LoadingState::SeparatingStems { .. } => "SeparatingStems".to_string(),
            LoadingState::LoadingAudio { .. } => "LoadingAudio".to_string(),
            LoadingState::GeneratingWaveforms { .. } => "GeneratingWaveforms".to_string(),
            LoadingState::Complete { .. } => "Complete".to_string(),
            LoadingState::Failed { .. } => "Failed".to_string(),
        }
    }

    pub fn display_message(&self) -> String {
        match self {
            LoadingState::Idle { .. } => "".to_string(),
            LoadingState::Downloading { stage_message, .. } => stage_message.clone(),
            LoadingState::SeparatingStems { stage_message, .. } => stage_message.clone(),
            LoadingState::LoadingAudio { stage_message, .. } => stage_message.clone(),
            LoadingState::GeneratingWaveforms { stage_message, .. } => stage_message.clone(),
            LoadingState::Complete { .. } => "Complete".to_string(),
            LoadingState::Failed { error_message, .. } => error_message.clone(),
        }
    }

    pub fn file_count(&self) -> usize {
        match self {
            LoadingState::Idle { file_count } => *file_count,
            LoadingState::Downloading { file_count, .. } => *file_count,
            LoadingState::SeparatingStems { file_count, .. } => *file_count,
            LoadingState::LoadingAudio { file_count, .. } => *file_count,
            LoadingState::GeneratingWaveforms { file_count, .. } => *file_count,
            LoadingState::Complete { file_count, .. } => *file_count,
            LoadingState::Failed { file_count, .. } => *file_count,
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(
            self,
            LoadingState::Downloading { .. }
                | LoadingState::SeparatingStems { .. }
                | LoadingState::LoadingAudio { .. }
                | LoadingState::GeneratingWaveforms { .. }
        )
    }

    pub fn to_qvariant_map(&self) -> QVariantMap {
        let mut map = QVariantMap::default();

        map.insert(
            "type".into(),
            QVariant::from(QString::from(self.as_string())),
        );
        map.insert(
            "file_count".into(),
            QVariant::from(self.file_count() as i32),
        );
        map.insert("is_loading".into(), QVariant::from(self.is_loading()));

        match self {
            LoadingState::Idle { .. } => {
                map.insert("stage_message".into(), QVariant::from(QString::from("")));
            }
            LoadingState::Downloading { stage_message, .. } => {
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from(stage_message.clone())),
                );
                map.insert("progress".into(), QVariant::from(0.0));
            }
            LoadingState::SeparatingStems {
                stage_message,
                progress,
                waveforms_completed,
                waveforms_total,
                ..
            } => {
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from(stage_message.clone())),
                );
                map.insert("progress".into(), QVariant::from(*progress));
                map.insert(
                    "waveforms_completed".into(),
                    QVariant::from(*waveforms_completed as i32),
                );
                map.insert(
                    "waveforms_total".into(),
                    QVariant::from(*waveforms_total as i32),
                );
            }
            LoadingState::LoadingAudio {
                stage_message,
                progress,
                ..
            } => {
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from(stage_message.clone())),
                );
                map.insert("progress".into(), QVariant::from(*progress));
            }
            LoadingState::GeneratingWaveforms {
                stage_message,
                progress,
                waveforms_completed,
                waveforms_total,
                ..
            } => {
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from(stage_message.clone())),
                );
                map.insert("progress".into(), QVariant::from(*progress));
                map.insert(
                    "waveforms_completed".into(),
                    QVariant::from(*waveforms_completed as i32),
                );
                map.insert(
                    "waveforms_total".into(),
                    QVariant::from(*waveforms_total as i32),
                );
            }
            LoadingState::Complete {
                all_waveforms_ready,
                ..
            } => {
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from("Complete")),
                );
                map.insert(
                    "all_waveforms_ready".into(),
                    QVariant::from(*all_waveforms_ready),
                );
            }
            LoadingState::Failed { error_message, .. } => {
                map.insert(
                    "error_message".into(),
                    QVariant::from(QString::from(error_message.clone())),
                );
                map.insert(
                    "stage_message".into(),
                    QVariant::from(QString::from(error_message.clone())),
                );
            }
        }

        map
    }
}

#[derive(Debug, Clone)]
pub struct LoadSongDisplay {
    pub artist_text: String,
    pub title_text: String,
    pub status_text: String,
    pub status_visible: bool,
}

impl LoadSongDisplay {
    pub fn new(loading_state: &LoadingState, metadata: &Option<SongMetadata>) -> Self {
        match loading_state {
            LoadingState::Idle { .. } => Self {
                artist_text: String::new(),
                title_text: String::new(),
                status_text: String::new(),
                status_visible: false,
            },
            LoadingState::Downloading { stage_message, .. } => Self {
                artist_text: String::new(),
                title_text: "Downloading audio...".to_string(),
                status_text: stage_message.clone(),
                status_visible: true,
            },
            LoadingState::SeparatingStems { progress, .. } => Self {
                artist_text: String::new(),
                title_text: "Separating audio...".to_string(),
                status_text: format!("{}%", (progress * 100.0).round() as i32),
                status_visible: true,
            },
            LoadingState::LoadingAudio { progress, .. } => Self {
                artist_text: String::new(),
                title_text: "Loading audio files...".to_string(),
                status_text: format!("{}%", (progress * 100.0).round() as i32),
                status_visible: true,
            },
            LoadingState::GeneratingWaveforms { progress, .. } => Self {
                artist_text: String::new(),
                title_text: "Analyzing audio...".to_string(),
                status_text: format!("{}%", (progress * 100.0).round() as i32),
                status_visible: true,
            },
            LoadingState::Complete { .. } => {
                let (title, artist) = if let Some(meta) = metadata {
                    (meta.display_title(), meta.display_artist())
                } else {
                    ("(title)".to_string(), "(artist)".to_string())
                };

                let status_text = if let Some(meta) = metadata {
                    meta.display_bpm().unwrap_or_else(|| "Ready".to_string())
                } else {
                    "Ready".to_string()
                };

                Self {
                    artist_text: artist,
                    title_text: title,
                    status_text,
                    status_visible: true,
                }
            }
            LoadingState::Failed { .. } => Self {
                artist_text: String::new(),
                title_text: "Failed to load".to_string(),
                status_text: String::new(),
                status_visible: false,
            },
        }
    }

    pub fn to_qvariant_map(&self) -> QVariantMap {
        let mut map = QVariantMap::default();
        map.insert(
            "artist_text".into(),
            QString::from(self.artist_text.as_str()).into(),
        );
        map.insert(
            "title_text".into(),
            QString::from(self.title_text.as_str()).into(),
        );
        map.insert(
            "status_text".into(),
            QString::from(self.status_text.as_str()).into(),
        );
        map.insert("status_visible".into(), self.status_visible.into());
        map
    }
}

#[derive(QObject)]
pub struct MultiBridge {
    base: qt_base_class!(trait QObject),

    // Multi-file audio engine and device management
    multi_engine: Option<MultiAudioEngine>,
    state_update_worker: Option<StateUpdateWorker>,
    device_manager: DeviceManager,

    // File paths for waveform analysis (paired with stem type)
    loaded_stems: Vec<StemmedFile>,
    engine_index_map: Vec<Option<usize>>,
    waveform_failures: Vec<bool>,

    // Note: Waveform data now stored in global WAVEFORM_REGISTRY

    // Original file path for beat analysis (when using single-file loading)
    original_file_for_beats: Option<String>,

    // Song metadata cache
    song_metadata: Option<SongMetadata>,

    // Download task tracking
    download_in_progress: bool,

    // Loading state tracking (internal enum)
    internal_loading_state: LoadingState,

    // Methods exposed to QML
    play: qt_method!(fn(&mut self)),
    pause: qt_method!(fn(&mut self)),
    stop: qt_method!(fn(&mut self)),
    seek: qt_method!(fn(&mut self, position: f64)),
    load_single_file: qt_method!(fn(&mut self, original_path: QString) -> bool),
    download_stems_from_url: qt_method!(fn(&mut self, url: QString)),
    get_player_info: qt_method!(fn(&self) -> QVariantMap),
    get_audio_devices: qt_method!(fn(&self) -> QVariantList),
    set_audio_device: qt_method!(fn(&mut self, device_name: QString) -> bool),
    set_master_volume: qt_method!(fn(&mut self, volume: f64)),
    set_playback_speed: qt_method!(fn(&mut self, speed: f64)),
    set_file_volume: qt_method!(fn(&mut self, file_index: i32, volume: f64)),
    toggle_mute: qt_method!(fn(&mut self, file_index: i32)),
    solo_track: qt_method!(fn(&mut self, file_index: i32)),
    reset_all_volumes: qt_method!(fn(&mut self)),
    mute_all: qt_method!(fn(&mut self)),
    unmute_all: qt_method!(fn(&mut self)),
    toggle_mute_all: qt_method!(fn(&mut self)),
    get_all_muted: qt_method!(fn(&self) -> bool),
    get_file_volume: qt_method!(fn(&self, file_index: i32) -> f64),
    get_file_mute: qt_method!(fn(&self, file_index: i32) -> bool),
    get_file_name: qt_method!(fn(&self, file_index: i32) -> QString),
    get_loading_state: qt_method!(fn(&self) -> QVariantMap),
    find_first_wav_in_directory: qt_method!(fn(&self, directory: QString) -> QString),
    update_state: qt_method!(fn(&mut self)),

    // Real waveform analysis methods
    check_waveform_progress: qt_method!(fn(&mut self)),
    is_waveform_ready: qt_method!(fn(&self, file_index: i32) -> bool),
    waveform_failed: qt_method!(fn(&self, file_index: i32) -> bool),

    // Properties exposed to QML
    pub is_playing: qt_property!(bool; NOTIFY is_playing_changed),
    pub current_position: qt_property!(f64; NOTIFY current_position_changed),
    pub duration: qt_property!(f64; NOTIFY duration_changed),
    pub master_volume: qt_property!(f64; NOTIFY master_volume_changed),
    pub playback_speed: qt_property!(f64; NOTIFY playback_speed_changed),
    pub file_count: qt_property!(i32; NOTIFY file_count_changed),
    pub current_device: qt_property!(QString; NOTIFY current_device_changed),
    pub loading_state: qt_property!(QVariantMap; NOTIFY loading_state_changed),

    pub load_song_display: qt_property!(QVariantMap; READ get_load_song_display NOTIFY loading_state_changed),

    // Signals (public fields)
    pub is_playing_changed: qt_signal!(),
    pub current_position_changed: qt_signal!(),
    pub duration_changed: qt_signal!(),
    pub master_volume_changed: qt_signal!(),
    pub playback_speed_changed: qt_signal!(),
    pub file_count_changed: qt_signal!(),
    pub current_device_changed: qt_signal!(),
    pub loading_state_changed: qt_signal!(),
    pub file_states_changed: qt_signal!(),
    pub playback_settings_changed: qt_signal!(),
    pub error_occurred: qt_signal!(message: QString),
}

impl Default for MultiBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiBridge {
    pub fn new() -> Self {
        let device_manager = DeviceManager::default();

        Self {
            base: Default::default(),
            multi_engine: None,
            state_update_worker: None,
            device_manager,
            loaded_stems: Vec::new(),
            engine_index_map: Vec::new(),
            waveform_failures: Vec::new(),
            original_file_for_beats: None,
            song_metadata: None,
            download_in_progress: false,
            internal_loading_state: LoadingState::Idle { file_count: 0 },
            play: Default::default(),
            pause: Default::default(),
            stop: Default::default(),
            seek: Default::default(),
            load_single_file: Default::default(),
            download_stems_from_url: Default::default(),
            get_player_info: Default::default(),
            get_audio_devices: Default::default(),
            set_audio_device: Default::default(),
            set_master_volume: Default::default(),
            set_playback_speed: Default::default(),
            set_file_volume: Default::default(),
            toggle_mute: Default::default(),
            solo_track: Default::default(),
            reset_all_volumes: Default::default(),
            mute_all: Default::default(),
            unmute_all: Default::default(),
            toggle_mute_all: Default::default(),
            get_all_muted: Default::default(),
            get_file_volume: Default::default(),
            get_file_mute: Default::default(),
            get_file_name: Default::default(),
            get_loading_state: Default::default(),
            find_first_wav_in_directory: Default::default(),
            update_state: Default::default(),
            check_waveform_progress: Default::default(),
            is_waveform_ready: Default::default(),
            waveform_failed: Default::default(),
            is_playing: false,
            current_position: 0.0,
            duration: 0.0,
            master_volume: 1.0,
            playback_speed: 1.0,
            file_count: 0,
            current_device: QString::default(),
            loading_state: QVariantMap::default(),
            load_song_display: QVariantMap::default(),
            is_playing_changed: Default::default(),
            current_position_changed: Default::default(),
            duration_changed: Default::default(),
            master_volume_changed: Default::default(),
            playback_speed_changed: Default::default(),
            file_count_changed: Default::default(),
            current_device_changed: Default::default(),
            loading_state_changed: Default::default(),
            file_states_changed: Default::default(),
            playback_settings_changed: Default::default(),
            error_occurred: Default::default(),
        }
    }

    fn stop_state_update_worker(&mut self) {
        self.state_update_worker = None;
    }

    fn start_state_update_worker(&mut self) {
        if self.state_update_worker.is_some() {
            return;
        }

        let Some(ref engine) = self.multi_engine else {
            return;
        };

        let supplier = engine.state_supplier();
        self.state_update_worker = Some(StateUpdateWorker::new(self, supplier));
    }

    fn restart_state_update_worker(&mut self) {
        self.stop_state_update_worker();
        self.start_state_update_worker();
    }

    pub fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing MultiBridge with multi-audio engine");

        // Initialize with default audio device
        if let Some(device) = self.device_manager.get_default_output_device() {
            let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            tracing::info!("Using default audio device: {}", device_name);

            match MultiAudioEngine::new(device) {
                Ok(engine) => {
                    self.current_device = QString::from(device_name);
                    self.multi_engine = Some(engine);
                    self.current_device_changed();
                    self.restart_state_update_worker();
                    tracing::info!("Multi-audio engine initialized successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize multi-audio engine: {}", e);
                    self.error_occurred(QString::from(format!(
                        "Audio initialization failed: {}",
                        e
                    )));
                }
            }
        } else {
            self.error_occurred(QString::from("No audio output devices found"));
        }

        Ok(())
    }

    // Audio command helpers
    fn send_audio_command(&self, command: MultiAudioCommand) {
        if let Some(ref engine) = self.multi_engine {
            if let Err(e) = engine.send_command(command) {
                tracing::error!("Failed to send multi-audio command: {}", e);
            }
        }
    }

    fn apply_state_snapshot(&mut self, state: MultiAudioState) {
        let mut changed = false;

        if self.is_playing != state.is_playing {
            self.is_playing = state.is_playing;
            self.is_playing_changed();
            changed = true;
            tracing::debug!("Play state updated: {}", state.is_playing);
        }

        if (self.current_position - state.position_seconds).abs() > f64::EPSILON {
            self.current_position = state.position_seconds;
            self.current_position_changed();
            changed = true;
        }

        if (self.duration - state.duration_seconds).abs() > f64::EPSILON {
            self.duration = state.duration_seconds;
            self.duration_changed();
            changed = true;
        }

        let new_volume = state.master_volume as f64;
        if (self.master_volume - new_volume).abs() > f64::EPSILON {
            self.master_volume = new_volume;
            self.master_volume_changed();
            changed = true;
        }

        let new_speed = state.playback_speed as f64;
        if (self.playback_speed - new_speed).abs() > f64::EPSILON {
            self.playback_speed = new_speed;
            self.playback_speed_changed();
            changed = true;
        }

        let new_file_count = state.file_count as i32;
        if self.file_count != new_file_count {
            self.file_count = new_file_count;
            self.file_count_changed();
            changed = true;
        }

        if changed {
            tracing::trace!(
                "State push: playing={}, pos={:.3}s, dur={:.3}s, files={}",
                state.is_playing,
                state.position_seconds,
                state.duration_seconds,
                state.file_count
            );
        }
    }

    fn update_state_from_engine(&mut self) {
        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            self.apply_state_snapshot(state);
        }
    }

    // Implement the qt_method functions
    fn play(&mut self) {
        tracing::info!("Play requested");
        self.send_audio_command(MultiAudioCommand::Play);
    }

    fn pause(&mut self) {
        tracing::info!("Pause requested");
        self.send_audio_command(MultiAudioCommand::Pause);
    }

    fn stop(&mut self) {
        tracing::info!("Stop requested");
        self.send_audio_command(MultiAudioCommand::Stop);
    }

    fn seek(&mut self, position: f64) {
        tracing::info!("Seek to position: {:.2}s", position);

        // Update UI state immediately for responsiveness
        self.current_position = position.max(0.0).min(self.duration);
        self.current_position_changed();

        // Send command to audio engine
        self.send_audio_command(MultiAudioCommand::Seek(position));
    }

    fn set_master_volume(&mut self, volume: f64) {
        tracing::info!("Set master volume to: {:.2}", volume);

        // Update UI state immediately for responsiveness
        self.master_volume = volume.clamp(0.0, 2.0);
        self.master_volume_changed();

        // Send command to audio engine
        self.send_audio_command(MultiAudioCommand::SetMasterVolume(volume as f32));
    }

    fn set_playback_speed(&mut self, speed: f64) {
        let clamped = (speed as f32).clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED);
        tracing::info!("Set playback speed to: {:.2}x", clamped);

        // Update UI state immediately for responsiveness
        self.playback_speed = clamped as f64;
        self.playback_speed_changed();

        self.send_audio_command(MultiAudioCommand::SetPlaybackSpeed(clamped));
    }

    fn set_file_volume(&mut self, file_index: i32, volume: f64) {
        let volume = volume.clamp(0.0, 2.0);
        tracing::info!("Set file {} volume to: {:.2}", file_index, volume);
        let Some(engine_index) = self.map_to_engine_index(file_index) else {
            return;
        };

        self.send_audio_command(MultiAudioCommand::SetVolume(engine_index, volume as f32));
        self.playback_settings_changed();
    }

    fn toggle_mute(&mut self, file_index: i32) {
        tracing::info!("Toggle file {} mute", file_index);
        let Some(engine_index) = self.map_to_engine_index(file_index) else {
            return;
        };

        self.send_audio_command(MultiAudioCommand::ToggleMute(engine_index));
        self.playback_settings_changed();
    }

    fn solo_track(&mut self, file_index: i32) {
        tracing::info!("Solo track {}", file_index);
        let Some(engine_index) = self.map_to_engine_index(file_index) else {
            return;
        };

        self.send_audio_command(MultiAudioCommand::SoloTrack(engine_index));
        self.playback_settings_changed();
    }

    fn reset_all_volumes(&mut self) {
        tracing::info!("Resetting all volumes to 100%");

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            let file_count = state.file_count;

            // Reset all file volumes to 1.0 (100%)
            for file_index in 0..file_count {
                self.send_audio_command(MultiAudioCommand::SetVolume(file_index, 1.0));
            }

            // Emit signal to update UI reactively
            self.playback_settings_changed();

            tracing::info!("Reset {} file volumes to 100%", file_count);
        }
    }

    fn mute_all(&mut self) {
        tracing::info!("Muting all tracks");

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            let file_count = state.file_count;

            // Mute all tracks by setting volume to 0
            for file_index in 0..file_count {
                self.send_audio_command(MultiAudioCommand::SetVolume(file_index, 0.0));
            }

            // Emit signal to update UI reactively
            self.playback_settings_changed();

            tracing::info!("Muted {} tracks", file_count);
        }
    }

    fn unmute_all(&mut self) {
        tracing::info!("Unmuting all tracks");

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            let file_count = state.file_count;

            // Unmute all tracks by setting volume to 1.0
            for file_index in 0..file_count {
                self.send_audio_command(MultiAudioCommand::SetVolume(file_index, 1.0));
            }

            // Emit signal to update UI reactively
            self.playback_settings_changed();

            tracing::info!("Unmuted {} tracks", file_count);
        }
    }

    fn toggle_mute_all(&mut self) {
        tracing::info!("Toggling mute for all tracks");

        // Check if all tracks are currently muted
        let all_muted = self.get_all_muted();

        if all_muted {
            self.unmute_all();
        } else {
            self.mute_all();
        }
    }

    fn resolve_music_directory() -> Result<PathBuf, String> {
        let home = std::env::var("HOME").map_err(|err| format!("HOME environment variable not set: {}", err))?;
        Ok(Path::new(&home).join("Music"))
    }

    /// Two-step pipeline:
    ///   1. yt-dlp pulls YouTube's highest-bitrate audio stream as Opus,
    ///      byte-for-byte (no re-encode). See `download_opus` for why Opus.
    ///   2. ffmpeg decodes Opus and resamples to 44.1 kHz s16 WAV.
    ///      htdemucs is trained at 44.1 kHz: it resamples its input to
    ///      44.1 kHz internally and emits stems at 44.1 kHz, so feeding
    ///      it 44.1 kHz directly avoids an extra resample inside demucs
    ///      and keeps the source WAV at the same rate as the stems
    ///      demucs will produce (which the player also depends on).
    fn run_yt_dlp_download(url: &str, music_dir: &Path, repo_root: &Path) -> Result<DownloadResult, String> {
        let opus_path = Self::download_opus(url, music_dir, repo_root)?;
        let wav_path = Self::resample_to_demucs_wav(&opus_path)?;
        Ok(DownloadResult {
            audio_path: wav_path.to_string_lossy().into_owned(),
        })
    }

    /// Download the source's Opus audio stream as-is (no re-encoding).
    ///
    /// We start with Opus because we want the highest-fidelity representation
    /// of the audio we can get from YouTube. YouTube serves multiple per-video
    /// audio streams via DASH (typically AAC at 49/130 kbps and Opus at
    /// 137-160 kbps); for music content the highest-bitrate stream is almost
    /// always Opus, and `--audio-format opus` tells yt-dlp to keep that stream
    /// bit-for-bit (just remuxed into an Ogg/Opus container). Any later step
    /// that wants WAV/FLAC works from this Opus master, so we never go through
    /// an unnecessary lossy hop.
    ///
    /// None of YouTube's streams are the uploader's original master — they're
    /// all lossy re-encodes of it — but among what's actually retrievable,
    /// this is the best.
    fn download_opus(url: &str, music_dir: &Path, repo_root: &Path) -> Result<PathBuf, String> {
        let output_template = music_dir.join("%(title)s.%(ext)s");
        let output_pattern = output_template.to_string_lossy().into_owned();

        let mut command = Command::new("uv");
        command
            .arg("run")
            .arg("yt-dlp")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("opus")
            .arg("--embed-metadata")
            .arg("--output")
            .arg(&output_pattern)
            .arg(url)
            .current_dir(repo_root);

        tracing::info!("Executing yt-dlp command: {:?}", command);

        let output = command
            .output()
            .map_err(|err| format!("Failed to run yt-dlp: {}", err))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            tracing::error!(
                "yt-dlp exited with status {:?}. stdout: {}; stderr: {}",
                output.status, stdout, stderr
            );
            return Err(format!(
                "yt-dlp failed: {}",
                if stderr.trim().is_empty() {
                    stdout.trim().to_string()
                } else {
                    stderr.trim().to_string()
                }
            ));
        }

        let audio_path = stdout
            .lines()
            .chain(stderr.lines())
            .find_map(|line| {
                line.trim()
                    .strip_prefix("[ExtractAudio] Destination: ")
                    .map(|rest| rest.trim().to_string())
            })
            .or_else(|| {
                std::fs::read_dir(music_dir)
                    .ok()
                    .and_then(|entries| {
                        let mut opus_files: Vec<_> = entries
                            .filter_map(|entry| entry.ok())
                            .filter(|entry| {
                                entry
                                    .path()
                                    .extension()
                                    .map(|ext| ext.eq_ignore_ascii_case("opus"))
                                    .unwrap_or(false)
                            })
                            .collect();
                        opus_files.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
                        opus_files
                            .into_iter()
                            .last()
                            .map(|entry| entry.path().to_string_lossy().into_owned())
                    })
            });

        match audio_path {
            Some(path) if Path::new(&path).exists() => {
                tracing::info!("yt-dlp download completed: {}", path);
                Ok(PathBuf::from(path))
            }
            Some(path) => {
                tracing::error!("yt-dlp reported output but file missing: {}", path);
                Err(format!("Downloaded file not found on disk: {}", path))
            }
            None => {
                tracing::error!(
                    "yt-dlp did not report a destination. stdout: {}; stderr: {}",
                    stdout,
                    stderr
                );
                Err("Failed to determine downloaded file path".to_string())
            }
        }
    }

    fn resample_to_demucs_wav(opus_path: &Path) -> Result<PathBuf, String> {
        // 44.1 kHz is htdemucs's native rate; see comment on run_yt_dlp_download.
        const DEMUCS_SAMPLE_RATE: &str = "44100";

        let wav_path = opus_path.with_extension("wav");
        let mut command = Command::new("ffmpeg");
        command
            .arg("-y")
            .arg("-loglevel")
            .arg("error")
            .arg("-i")
            .arg(opus_path)
            .arg("-ar")
            .arg(DEMUCS_SAMPLE_RATE)
            .arg("-c:a")
            .arg("pcm_s16le")
            .arg(&wav_path);

        tracing::info!("Executing ffmpeg resample command: {:?}", command);

        let output = command
            .output()
            .map_err(|err| format!("Failed to run ffmpeg: {}", err))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                "ffmpeg exited with status {:?}. stderr: {}",
                output.status,
                stderr
            );
            return Err(format!("ffmpeg resample failed: {}", stderr.trim()));
        }

        if !wav_path.exists() {
            return Err(format!(
                "ffmpeg reported success but output missing: {}",
                wav_path.display()
            ));
        }

        tracing::info!("Resampled to 44.1 kHz WAV: {}", wav_path.display());
        Ok(wav_path)
    }

    fn spawn_download_task(
        &mut self,
        url: String,
        stage_message: String,
    ) -> Result<(), String> {
        let music_dir = Self::resolve_music_directory()?;
        if let Err(err) = std::fs::create_dir_all(&music_dir) {
            return Err(format!(
                "Failed to ensure music directory {} exists: {}",
                music_dir.display(),
                err
            ));
        }

        self.update_loading_state(LoadingState::Downloading {
            file_count: 0,
            stage_message: stage_message.clone(),
        });

        self.download_in_progress = true;

        let command_url = url.clone();
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bridge_ptr = QPointer::from(&*self);
        let music_dir_clone = music_dir.clone();
        let download_callback = queued_callback(move |outcome: Result<DownloadResult, String>| {
            if let Some(pinned) = bridge_ptr.as_pinned() {
                let mut bridge_ref = pinned.borrow_mut();
                bridge_ref.download_in_progress = false;
                bridge_ref.handle_download_result(outcome);
            }
        });

        thread::spawn(move || {
            let outcome = Self::run_yt_dlp_download(&command_url, &music_dir_clone, &repo_root);
            download_callback(outcome);
        });

        Ok(())
    }

    fn spawn_separation_task(
        &mut self,
        source_path: String,
        stems_root: PathBuf,
    ) -> Result<(), String> {
        tracing::info!(
            "Starting stem separation for {} into {}",
            source_path,
            stems_root.display()
        );

        self.update_loading_state(LoadingState::SeparatingStems {
            file_count: 1,
            stage_message: "Separating audio...".to_string(),
            progress: 0.0,
            waveforms_completed: 0,
            waveforms_total: 1,
        });

        let bridge_ptr = QPointer::from(&*self);
        let callback_source_path = source_path.clone();
        let separation_callback = queued_callback(move |outcome: Result<SeparationResult, String>| {
            if let Some(pinned) = bridge_ptr.as_pinned() {
                let mut bridge_ref = pinned.borrow_mut();
                bridge_ref.handle_separation_result(outcome, callback_source_path.clone());
            }
        });

        thread::spawn(move || {
            let outcome = analysis::separate_stems(&source_path, &stems_root)
                .map_err(|err| err.to_string())
                .and_then(|result| {
                    if result.success {
                        Ok(result)
                    } else {
                        Err(result
                            .error
                            .clone()
                            .unwrap_or_else(|| "Stem separation failed".to_string()))
                    }
                });

            separation_callback(outcome);
        });

        Ok(())
    }

    fn get_all_muted(&self) -> bool {
        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();

            // Return true if all tracks are muted (volume == 0)
            if state.file_count == 0 {
                return false;
            }

            for i in 0..state.file_count {
                if let Some(&volume) = state.file_volumes.get(i) {
                    if volume > 0.0 {
                        return false; // At least one track is not muted
                    }
                }
            }

            true // All tracks are muted
        } else {
            false
        }
    }

    fn download_stems_from_url(&mut self, url: QString) {
        let url_string = url.to_string();
        let trimmed = url_string.trim();

        if trimmed.is_empty() {
            self.error_occurred(QString::from("Please provide a link to download."));
            return;
        }

        if self.internal_loading_state.is_loading() || self.download_in_progress {
            self.error_occurred(QString::from("Another load is already in progress."));
            return;
        }

        let parsed = match Url::parse(trimmed) {
            Ok(url) => url,
            Err(err) => {
                self.error_occurred(QString::from(format!("Invalid URL: {}", err)));
                return;
            }
        };

        match parsed.scheme() {
            "http" | "https" => {}
            _ => {
                self.error_occurred(QString::from("Only HTTP/HTTPS links are supported."));
                return;
            }
        }

        let stage_message = parsed
            .host_str()
            .map(|host| format!("Downloading from {}", host))
            .unwrap_or_else(|| "Downloading audio...".to_string());

        tracing::info!("Starting yt-dlp download for URL: {}", trimmed);

        if let Err(err) = self.spawn_download_task(trimmed.to_string(), stage_message) {
            tracing::error!("Failed to start download task: {}", err);
            self.error_occurred(QString::from(err.clone()));
            self.update_loading_state(LoadingState::Failed {
                error_message: err,
                file_count: 0,
            });
        }
    }

    fn load_single_file(&mut self, original_path: QString) -> bool {
        tracing::info!("Loading single file with stem discovery: {}", original_path);

        // Parse the file:// URL and decode percent-encoding (same logic as load_files)
        let original_path_str = if original_path.to_string().starts_with("file://") {
            let url_string = original_path.to_string();
            let url = match Url::parse(&url_string) {
                Ok(url) => url,
                Err(e) => {
                    self.error_occurred(QString::from(format!("Invalid file URL: {}", url_string)));
                    return false;
                }
            };

            match url.to_file_path() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(_) => {
                    self.error_occurred(QString::from(format!("Invalid file URL: {}", url_string)));
                    return false;
                }
            }
        } else {
            original_path.to_string() // Plain file path
        };

        // Validate original file following existing patterns
        if original_path_str.is_empty() {
            self.error_occurred(QString::from("File path is empty"));
            return false;
        }

        if !Path::new(&original_path_str).exists() {
            self.error_occurred(QString::from(format!(
                "File does not exist: {}",
                original_path_str
            )));
            return false;
        }

        let stems_root = Path::new(&original_path_str)
            .parent()
            .map(|parent| parent.join("stems"))
            .unwrap_or_else(|| PathBuf::from("stems"));

        // Discover stems using hardcoded pattern
        let discovered_stems = match StemDiscovery::discover_stems_from_file(&original_path_str) {
            Ok(stems) => stems,
            Err(e) => {
                self.error_occurred(QString::from(format!("Stem discovery failed: {}", e)));
                return false;
            }
        };

        // Convert to file paths in UI rendering order (vocals, bass, drums, other)
        let mut valid_stems: Vec<StemmedFile> = discovered_stems
            .into_iter()
            .map(|stem_match| StemmedFile {
                stem: stem_match.stem_type,
                path: stem_match.file_path,
                exists: stem_match.exists,
            })
            .collect();

        valid_stems.sort_by_key(|entry| entry.stem.into_index());

        if valid_stems.is_empty() {
            self.error_occurred(QString::from("No valid stem files found"));
            return false;
        }

        let all_stems_exist = valid_stems
            .iter()
            .all(|entry| entry.exists && Path::new(&entry.path).exists());

        if !all_stems_exist {
            tracing::info!(
                "Not all stems are present on disk; triggering separation for {}",
                original_path_str
            );

            self.song_metadata = None;
            self.original_file_for_beats = Some(original_path_str.clone());
            if let Err(err) = self.spawn_separation_task(original_path_str.clone(), stems_root) {
                tracing::error!("Failed to start separation from load_single_file: {}", err);
                self.error_occurred(QString::from(err.clone()));
                self.update_loading_state(LoadingState::Failed {
                    error_message: err,
                    file_count: 0,
                });
                return false;
            }

            return true;
        }

        tracing::info!("Found {} ready stem files", valid_stems.len());

        // Store original file path for beat analysis
        self.original_file_for_beats = Some(original_path_str.clone());

        // Build a list of existing stem paths for metadata fallback
        let stem_paths: Vec<String> = valid_stems
            .iter()
            .filter(|entry| entry.exists && Path::new(&entry.path).exists())
            .map(|entry| entry.path.clone())
            .collect();

        // Extract metadata from the original file, with fallback to stems
        match extract_metadata(&original_path_str)
            .or_else(|err| {
                tracing::warn!("Metadata from original file failed: {}", err);
                extract_metadata_from_first_file(&stem_paths)
            })
        {
            Ok(metadata) => {
                tracing::info!(
                    "Metadata ready: title={:?}, artist={:?}",
                    metadata.title,
                    metadata.artist
                );
                self.song_metadata = Some(metadata);
            }
            Err(e) => {
                tracing::warn!("Failed to extract metadata from stems: {}", e);
                self.song_metadata = Some(SongMetadata::default());
            }
        }

        // Set loading state
        tracing::info!(
            "RUST TRANSITION DEBUG: Starting LoadingAudio phase for {} stems",
            valid_stems.len()
        );
        self.update_loading_state(LoadingState::LoadingAudio {
            file_count: valid_stems.len(),
            stage_message: "Loading audio files...".to_string(),
            progress: 0.0,
        });

        // Clear waveform registry
        WAVEFORM_REGISTRY.lock().unwrap().clear();

        // Get default audio device
        let device = match self.device_manager.get_default_output_device() {
            Some(device) => device,
            None => {
                self.error_occurred(QString::from("No audio device available"));
                self.update_loading_state(LoadingState::Failed {
                    error_message: "No audio device available".to_string(),
                    file_count: valid_stems.len(),
                });
                return false;
            }
        };

        let mut stem_load_list: Vec<String> = Vec::new();
        let mut engine_index_map: Vec<Option<usize>> = Vec::with_capacity(valid_stems.len());
        let mut initial_waveform_failures: Vec<bool> = Vec::with_capacity(valid_stems.len());

        for entry in &valid_stems {
            let exists = entry.exists && Path::new(&entry.path).exists();
            initial_waveform_failures.push(!exists);
            if exists {
                engine_index_map.push(Some(stem_load_list.len()));
                stem_load_list.push(entry.path.clone());
            } else {
                engine_index_map.push(None);
            }
        }

        self.engine_index_map = engine_index_map;
        self.waveform_failures = initial_waveform_failures;

        // Create multi-engine with validated files
        match MultiAudioEngine::new_with_files(device, stem_load_list) {
            Ok(engine) => {
                self.stop_state_update_worker();
                self.multi_engine = Some(engine);
                self.start_state_update_worker();

                // Apply default per-stem volumes on song load.
                self.set_file_volume(StemType::Drums.into_index() as i32, 0.5);
                self.set_file_volume(StemType::Other.into_index() as i32, 1.5);

                // Start waveform generation phase
                tracing::info!(
                    "RUST TRANSITION DEBUG: Starting GeneratingWaveforms phase for {} files",
                    valid_stems.len()
                );
                self.update_loading_state(LoadingState::GeneratingWaveforms {
                    file_count: valid_stems.len(),
                    stage_message: "Generating waveforms...".to_string(),
                    progress: 0.0,
                    waveforms_completed: 0,
                    waveforms_total: valid_stems.len(),
                });

                // Update state immediately
                self.update_state_from_engine();

                // Store file paths for waveform analysis
                self.loaded_stems = valid_stems.clone();

                // Metadata was already extracted from original file above

                // Start real waveform generation in background
                self.start_background_waveform_generation();

                // Audio loading complete, but waveforms still generating
                // Don't set to Complete yet - that happens when waveforms finish

                true
            }
            Err(e) => {
                // Clear loading state with error stage
                self.update_loading_state(LoadingState::Failed {
                    error_message: format!("Audio loading failed: {}", e),
                    file_count: 0,
                });

                // Signal error to UI (existing pattern)
                self.error_occurred(QString::from(format!("Failed to load audio files: {}", e)));

                false
            }
        }
    }

    fn get_player_info(&self) -> QVariantMap {
        let mut map = QVariantMap::default();
        map.insert("version".into(), QVariant::from(QString::from("2.0.0")));
        map.insert(
            "backend".into(),
            QVariant::from(QString::from("multi-cpal")),
        );
        map.insert("supports_multi_file".into(), QVariant::from(true));

        if let Some(ref engine) = self.multi_engine {
            map.insert(
                "current_device".into(),
                QVariant::from(QString::from(engine.get_device_name())),
            );
            map.insert("multi_engine_active".into(), QVariant::from(true));
        } else {
            map.insert("multi_engine_active".into(), QVariant::from(false));
        }

        map
    }

    fn get_audio_devices(&self) -> QVariantList {
        let mut devices = QVariantList::default();

        match self.device_manager.list_output_devices() {
            Ok(device_list) => {
                for device in device_list {
                    let mut device_map = QVariantMap::default();
                    device_map.insert("name".into(), QVariant::from(QString::from(device.name)));
                    device_map.insert("is_default".into(), QVariant::from(device.is_default));
                    devices.push(QVariant::from(device_map));
                }
            }
            Err(e) => {
                tracing::error!("Failed to list audio devices: {}", e);
            }
        }

        devices
    }

    fn set_audio_device(&mut self, device_name: QString) -> bool {
        let device_name_str = device_name.to_string();
        tracing::info!("Setting audio device to: {}", device_name_str);

        // Find the device
        match self.device_manager.list_output_devices() {
            Ok(devices) => {
                for device_info in devices {
                    if device_info.name == device_name_str {
                        // Try to get the actual device
                        if let Ok(device_iter) = self.device_manager.host.output_devices() {
                            for device in device_iter {
                                if let Ok(name) = device.name() {
                                    if name == device_name_str {
                                        // Create new multi-audio engine with this device
                                        match MultiAudioEngine::new(device) {
                                            Ok(engine) => {
                                                self.multi_engine = Some(engine);
                                                self.current_device = device_name;
                                                self.current_device_changed();
                                                tracing::info!(
                                                    "Successfully switched to device: {}",
                                                    device_name_str
                                                );
                                                return true;
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to create multi-audio engine for device {}: {}", device_name_str, e);
                                                self.error_occurred(QString::from(format!(
                                                    "Failed to switch device: {}",
                                                    e
                                                )));
                                                return false;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.error_occurred(QString::from(format!(
                    "Device '{}' not found",
                    device_name_str
                )));
                false
            }
            Err(e) => {
                tracing::error!("Failed to list devices: {}", e);
                self.error_occurred(QString::from("Failed to list audio devices"));
                false
            }
        }
    }

    fn get_file_volume(&self, file_index: i32) -> f64 {
        let Some(engine_index) = self.map_to_engine_index(file_index) else {
            return 1.0;
        };

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            state.file_volumes.get(engine_index).copied().unwrap_or(1.0) as f64
        } else {
            1.0
        }
    }

    fn get_file_mute(&self, file_index: i32) -> bool {
        let Some(engine_index) = self.map_to_engine_index(file_index) else {
            return false;
        };

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            state.file_mutes.get(engine_index).copied().unwrap_or(false)
        } else {
            false
        }
    }

    // Public helper method for periodic state updates
    pub fn update_state(&mut self) {
        self.update_state_from_engine();
    }

    fn handle_download_result(&mut self, result: Result<DownloadResult, String>) {
        match result {
            Ok(download) => {
                tracing::info!(
                    "Download complete, preparing separation for {}",
                    download.audio_path
                );

                if !Path::new(&download.audio_path).exists() {
                    let message = format!("Downloaded file missing: {}", download.audio_path);
                    self.error_occurred(QString::from(message.clone()));
                    self.update_loading_state(LoadingState::Failed {
                        error_message: message,
                        file_count: 0,
                    });
                    return;
                }

                let stems_root = Path::new(&download.audio_path)
                    .parent()
                    .map(|parent| parent.join("stems"))
                    .unwrap_or_else(|| PathBuf::from("stems"));

                if let Err(err) = self.spawn_separation_task(download.audio_path.clone(), stems_root) {
                    tracing::error!("Failed to start separation task: {}", err);
                    self.error_occurred(QString::from(err.clone()));
                    self.update_loading_state(LoadingState::Failed {
                        error_message: err,
                        file_count: 0,
                    });
                }
            }
            Err(err_msg) => {
                tracing::error!("Download failed: {}", err_msg);
                self.error_occurred(QString::from(err_msg.clone()));
                self.update_loading_state(LoadingState::Failed {
                    error_message: err_msg,
                    file_count: 0,
                });
            }
        }
    }

    fn handle_separation_result(&mut self, result: Result<SeparationResult, String>, original_path: String) {
        match result {
            Ok(separation) => {
                let stem_dir = match separation.stem_dir.as_deref() {
                    Some(path) => path.to_string(),
                    None => {
                        let message = "Separation completed without reporting stem directory".to_string();
                        tracing::error!("{}", message);
                        self.error_occurred(QString::from(message.clone()));
                        self.update_loading_state(LoadingState::Failed {
                            error_message: message,
                            file_count: 0,
                        });
                        return;
                    }
                };

                tracing::info!(
                    "Stem separation successful: {} (files: {:?})",
                    stem_dir,
                    separation.generated_files
                );

                self.original_file_for_beats = Some(original_path.clone());

                let path_qstring = QString::from(original_path.as_str());
                if !self.load_single_file(path_qstring) {
                    let message = "Failed to load stems after separation".to_string();
                    tracing::error!("{}", message);
                    self.error_occurred(QString::from(message.clone()));
                    self.update_loading_state(LoadingState::Failed {
                        error_message: message,
                        file_count: 0,
                    });
                }
            }
            Err(err_msg) => {
                tracing::error!("Stem separation failed: {}", err_msg);
                self.error_occurred(QString::from(err_msg.clone()));
                self.update_loading_state(LoadingState::Failed {
                    error_message: err_msg,
                    file_count: 0,
                });
            }
        }
    }

    // Real waveform generation methods
    fn start_background_waveform_generation(&mut self) {
        let stem_count = self.loaded_stems.len();
        tracing::info!(
            "Starting background waveform generation for {} stems",
            stem_count
        );

        if stem_count == 0 {
            return;
        }

        WAVEFORM_REGISTRY.lock().unwrap().clear();

        let progress_pointer: QPointer<MultiBridge> = QPointer::from(&*self);
        let progress_callback = queued_callback(move |()| {
            if let Some(pinned) = progress_pointer.as_pinned() {
                pinned.borrow_mut().check_waveform_progress();
            }
        });

        let progress_cb_missing = progress_callback.clone();

        let mut stems_to_process: Vec<StemmedFile> = Vec::new();
        for (idx, entry) in self.loaded_stems.iter().enumerate() {
            let path = Path::new(&entry.path);
            if !entry.exists || !path.exists() {
                tracing::warn!(
                    "Waveform source missing for {:?}: {}",
                    entry.stem,
                    entry.path
                );
                self.waveform_failures[idx] = true;
                Self::register_empty_waveform(entry.stem);
                progress_cb_missing(());
            } else {
                self.waveform_failures[idx] = false;
                stems_to_process.push(entry.clone());
            }
        }

        let original_for_beats = self.original_file_for_beats.clone();
        let progress_cb_thread = progress_callback.clone();

        thread::spawn(move || {
            let (shared_beats_vec, shared_tempo) = match original_for_beats.as_ref() {
                Some(original) => {
                    tracing::info!("Starting beat detection for original track: {}", original);

                    if Path::new(original).exists() {
                        match WavLoader::load_file_mapped(original) {
                            Ok(original_audio) => {
                                match crate::analysis::python_beat_detection::detect_beats_python(
                                    &original_audio,
                                ) {
                                    Ok(res) if res.success => {
                                        tracing::info!(
                                            "Beat detection succeeded: {} beats at {:.1} BPM",
                                            res.beat_count,
                                            res.tempo
                                        );
                                        (res.beat_timestamps, Some(res.tempo))
                                    }
                                    Ok(res) => {
                                        tracing::warn!(
                                            "Beat detection reported failure for {}: {:?}",
                                            original,
                                            res.error
                                        );
                                        (Vec::new(), None)
                                    }
                                    Err(err) => {
                                        tracing::error!(
                                            "Beat detection error for {}: {}",
                                            original,
                                            err
                                        );
                                        (Vec::new(), None)
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::error!(
                                    "Failed to load original audio for beats {}: {}",
                                    original,
                                    err
                                );
                                (Vec::new(), None)
                            }
                        }
                    } else {
                        tracing::warn!("Original track for beat detection missing: {}", original);
                        (Vec::new(), None)
                    }
                }
                None => (Vec::new(), None),
            };

            let shared_beats = Arc::new(shared_beats_vec);

            stems_to_process.par_iter().for_each(|entry| {
                tracing::info!(
                    "Preparing raw waveform for stem {:?}: {}",
                    entry.stem,
                    entry.path
                );

                match WavLoader::load_mono_i16(&entry.path) {
                    Ok((mono, spec)) => {
                        let beat_timestamps = shared_beats.as_ref().clone();
                        let tempo = shared_tempo;

                        let duration_seconds = mono.len() as f64 / spec.sample_rate as f64;
                        let lod = RawWaveformData::build_lod_pyramid(&mono, 256, 65536);

                        let raw = RawWaveformData {
                            sample_rate: spec.sample_rate,
                            channels: spec.channels,
                            duration_seconds,
                            mono,
                            beat_timestamps,
                            tempo,
                            lod,
                        };

                        WAVEFORM_REGISTRY
                            .lock()
                            .unwrap()
                            .set_waveform_data(entry.stem, raw);
                        progress_cb_thread(());
                    }
                    Err(e) => {
                        tracing::error!("Failed to load mono samples for {}: {}", entry.path, e);
                        MultiBridge::register_empty_waveform(entry.stem);
                        progress_cb_thread(());
                    }
                }
            });

            progress_cb_thread(());
            tracing::info!("Background raw waveform preparation completed");
        });
    }

    fn check_waveform_progress(&mut self) {
        // Only run if we're in the GeneratingWaveforms state
        let (current_file_count, current_total) = match &self.internal_loading_state {
            LoadingState::GeneratingWaveforms {
                file_count,
                waveforms_total,
                ..
            } => (*file_count, *waveforms_total),
            _ => return, // Not generating waveforms, nothing to do
        };

        let registry = WAVEFORM_REGISTRY.lock().unwrap();
        let ready_count = registry.ready_count();
        let total_count = self.loaded_stems.len();
        drop(registry);

        // Calculate progress
        let progress = if total_count > 0 {
            ready_count as f64 / total_count as f64
        } else {
            0.0
        };

        // Update the GeneratingWaveforms state with current progress
        self.update_loading_state(LoadingState::GeneratingWaveforms {
            file_count: current_file_count,
            stage_message: format!("Generating waveforms... ({}/{})", ready_count, total_count),
            progress,
            waveforms_completed: ready_count,
            waveforms_total: total_count,
        });

        // Check if all waveforms are complete
        let all_ready = ready_count == total_count && total_count > 0;
        if all_ready {
            tracing::info!("All {} waveforms generated successfully", total_count);

            // Complete main loading - waveforms are ready
            self.update_loading_state(LoadingState::Complete {
                file_count: current_file_count,
                all_waveforms_ready: true,
            });

            // Notify QML that waveforms are ready
            self.file_states_changed();
        }
    }

    fn is_waveform_ready(&self, file_index: i32) -> bool {
        if file_index < 0 {
            return false;
        }

        let idx = file_index as usize;
        let Some(entry) = self.loaded_stems.get(idx) else {
            return false;
        };
        WAVEFORM_REGISTRY.lock().unwrap().is_ready(entry.stem)
    }

    fn waveform_failed(&self, file_index: i32) -> bool {
        if file_index < 0 {
            return false;
        }

        let idx = file_index as usize;
        self.waveform_failures.get(idx).copied().unwrap_or(false)
    }

    fn map_to_engine_index(&self, file_index: i32) -> Option<usize> {
        if file_index < 0 {
            return None;
        }
        let idx = file_index as usize;
        self.engine_index_map.get(idx).copied().flatten()
    }

    fn register_empty_waveform(stem: StemType) {
        let raw = RawWaveformData {
            sample_rate: 44_100,
            channels: 2,
            duration_seconds: 0.0,
            mono: Vec::new(),
            beat_timestamps: Vec::new(),
            tempo: None,
            lod: Vec::new(),
        };
        WAVEFORM_REGISTRY
            .lock()
            .unwrap()
            .set_waveform_data(stem, raw);
    }

    fn get_file_name(&self, file_index: i32) -> QString {
        if file_index < 0 {
            tracing::debug!("get_file_name called with negative index: {}", file_index);
            return QString::from("Unknown");
        }

        let idx = file_index as usize;

        if let Some(entry) = self.loaded_stems.get(idx) {
            return QString::from(entry.stem.display_name());
        }

        if let Some(ref engine) = self.multi_engine {
            let state = engine.get_state();
            tracing::debug!(
                "get_file_name: engine state has {} file names",
                state.file_names.len()
            );

            if let Some(file_path) = state.file_names.get(idx) {
                tracing::debug!(
                    "get_file_name: file_path for index {}: {}",
                    file_index,
                    file_path
                );

                // Extract filename from full path and remove extension
                if let Some(filename) = std::path::Path::new(file_path).file_stem() {
                    if let Some(name_str) = filename.to_str() {
                        tracing::debug!("get_file_name: extracted filename: {}", name_str);
                        return QString::from(name_str);
                    }
                }
                // Fallback: return the full path if we can't extract filename
                tracing::debug!("get_file_name: using full path as fallback");
                return QString::from(file_path.clone());
            } else {
                tracing::debug!(
                    "get_file_name: index {} out of bounds for {} files",
                    file_index,
                    state.file_names.len()
                );
            }
        } else {
            tracing::debug!("get_file_name: multi_engine is None");
        }

        QString::from("Unknown")
    }

    fn get_load_song_display(&self) -> QVariantMap {
        let display = LoadSongDisplay::new(&self.internal_loading_state, &self.song_metadata);
        display.to_qvariant_map()
    }

    fn get_loading_state(&self) -> QVariantMap {
        self.internal_loading_state.to_qvariant_map()
    }

    fn find_first_wav_in_directory(&self, directory: QString) -> QString {
        let dir_path = directory.to_string();

        tracing::debug!("Searching for .wav files in directory: {}", dir_path);

        match std::fs::read_dir(&dir_path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if let Some(extension) = path.extension() {
                            if extension.to_string_lossy().to_lowercase() == "wav" {
                                if let Some(path_str) = path.to_str() {
                                    tracing::debug!("Found .wav file: {}", path_str);
                                    return QString::from(path_str);
                                }
                            }
                        }
                    }
                }
                tracing::debug!("No .wav files found in directory: {}", dir_path);
            }
            Err(e) => {
                tracing::warn!("Failed to read directory {}: {}", dir_path, e);
            }
        }

        QString::default() // Return empty string if no .wav files found
    }

    // Helper method to update loading state and emit all necessary signals
    fn update_loading_state(&mut self, new_state: LoadingState) {
        if self.internal_loading_state != new_state {
            tracing::info!(
                "RUST STATE DEBUG: Loading state changing from {:?} to {:?}",
                self.internal_loading_state,
                new_state
            );

            self.internal_loading_state = new_state.clone();

            // Update the Qt property with the rich state data
            self.loading_state = new_state.to_qvariant_map();
            tracing::info!(
                "RUST STATE DEBUG: Qt property updated, QVariantMap: {:?}",
                self.loading_state
            );

            // Emit signal for UI updates
            tracing::info!("RUST STATE DEBUG: Emitting loading_state_changed signal");
            self.loading_state_changed();
            tracing::info!("RUST STATE DEBUG: Signal emission complete");
        } else {
            tracing::debug!(
                "RUST STATE DEBUG: State unchanged, staying at {:?}",
                new_state
            );
        }
    }
}
