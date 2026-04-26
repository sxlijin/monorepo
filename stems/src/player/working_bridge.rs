use crate::audio::{AudioCommand, AudioEngine, DeviceManager};
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use qmetaobject::*;
use std::path::Path;

#[derive(QObject)]
pub struct PlayerBridge {
    base: qt_base_class!(trait QObject),

    // Audio engine and device management
    audio_engine: Option<AudioEngine>,
    device_manager: DeviceManager,

    // Methods exposed to QML
    play: qt_method!(fn(&mut self)),
    pause: qt_method!(fn(&mut self)),
    stop: qt_method!(fn(&mut self)),
    seek: qt_method!(fn(&mut self, position: f64)),
    load_file: qt_method!(fn(&mut self, path: QString) -> bool),
    get_player_info: qt_method!(fn(&self) -> QVariantMap),
    get_audio_devices: qt_method!(fn(&self) -> QVariantList),
    set_audio_device: qt_method!(fn(&mut self, device_name: QString) -> bool),
    set_volume: qt_method!(fn(&mut self, volume: f64)),
    update_state: qt_method!(fn(&mut self)),

    // Properties exposed to QML
    pub is_playing: qt_property!(bool; NOTIFY is_playing_changed),
    pub current_position: qt_property!(f64; NOTIFY current_position_changed),
    pub duration: qt_property!(f64; NOTIFY duration_changed),
    pub current_file: qt_property!(QString; NOTIFY current_file_changed),
    pub volume: qt_property!(f64; NOTIFY volume_changed),
    pub current_device: qt_property!(QString; NOTIFY current_device_changed),

    // Signals (public fields)
    pub is_playing_changed: qt_signal!(),
    pub current_position_changed: qt_signal!(),
    pub duration_changed: qt_signal!(),
    pub current_file_changed: qt_signal!(),
    pub volume_changed: qt_signal!(),
    pub current_device_changed: qt_signal!(),
    pub error_occurred: qt_signal!(message: QString),
}

impl Default for PlayerBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerBridge {
    pub fn new() -> Self {
        let device_manager = DeviceManager::default();

        Self {
            base: Default::default(),
            audio_engine: None,
            device_manager,
            play: Default::default(),
            pause: Default::default(),
            stop: Default::default(),
            seek: Default::default(),
            load_file: Default::default(),
            get_player_info: Default::default(),
            get_audio_devices: Default::default(),
            set_audio_device: Default::default(),
            set_volume: Default::default(),
            update_state: Default::default(),
            is_playing: false,
            current_position: 0.0,
            duration: 0.0,
            current_file: QString::default(),
            volume: 1.0,
            current_device: QString::default(),
            is_playing_changed: Default::default(),
            current_position_changed: Default::default(),
            duration_changed: Default::default(),
            current_file_changed: Default::default(),
            volume_changed: Default::default(),
            current_device_changed: Default::default(),
            error_occurred: Default::default(),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing PlayerBridge with audio engine");

        // Initialize with default audio device
        if let Some(device) = self.device_manager.get_default_output_device() {
            let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            tracing::info!("Using default audio device: {}", device_name);

            match AudioEngine::new(device) {
                Ok(engine) => {
                    self.current_device = QString::from(device_name);
                    self.audio_engine = Some(engine);
                    self.current_device_changed();
                    tracing::info!("Audio engine initialized successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize audio engine: {}", e);
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
    fn send_audio_command(&self, command: AudioCommand) {
        if let Some(ref engine) = self.audio_engine {
            if let Err(e) = engine.send_command(command) {
                tracing::error!("Failed to send audio command: {}", e);
            }
        }
    }

    fn update_state_from_engine(&mut self) {
        if let Some(ref engine) = self.audio_engine {
            let state = engine.get_state();

            let mut changed = false;

            // Update playing state to match audio engine
            if self.is_playing != state.is_playing {
                self.is_playing = state.is_playing;
                self.is_playing_changed();
                changed = true;
                tracing::debug!("Play state updated: {}", state.is_playing);
            }

            // Update position frequently for smooth seeking display
            if (self.current_position - state.position_seconds).abs() > 0.05 {
                self.current_position = state.position_seconds;
                self.current_position_changed();
                changed = true;
            }

            // Always update duration from engine (file loading updates)
            if (self.duration - state.duration_seconds).abs() > 0.1 {
                self.duration = state.duration_seconds;
                self.duration_changed();
                changed = true;
            }

            // Only update volume if there's a significant difference (avoid UI conflicts)
            let new_volume = state.volume as f64;
            if (self.volume - new_volume).abs() > 0.05 {
                self.volume = new_volume;
                self.volume_changed();
                changed = true;
            }

            if changed {
                tracing::debug!(
                    "State reconciled: playing={}, pos={:.2}s, dur={:.2}s, vol={:.2}",
                    state.is_playing,
                    state.position_seconds,
                    state.duration_seconds,
                    state.volume
                );
            }
        }
    }

    // Implement the qt_method functions
    fn play(&mut self) {
        tracing::info!("Play requested");

        // Send command to audio engine
        self.send_audio_command(AudioCommand::Play);
    }

    fn pause(&mut self) {
        tracing::info!("Pause requested");

        // Send command to audio engine
        self.send_audio_command(AudioCommand::Pause);
    }

    fn stop(&mut self) {
        tracing::info!("Stop requested");

        // Send command to audio engine
        self.send_audio_command(AudioCommand::Stop);
    }

    fn seek(&mut self, position: f64) {
        tracing::info!("Seek to position: {:.2}s", position);

        // Update UI state immediately for responsiveness
        self.current_position = position.max(0.0).min(self.duration);
        self.current_position_changed();

        // Send command to audio engine
        self.send_audio_command(AudioCommand::Seek(position));
    }

    fn set_volume(&mut self, volume: f64) {
        tracing::info!("Set volume to: {:.2}", volume);

        // Update UI state immediately for responsiveness
        self.volume = volume.clamp(0.0, 2.0);
        self.volume_changed();

        // Send command to audio engine
        self.send_audio_command(AudioCommand::SetVolume(volume as f32));
    }

    fn load_file(&mut self, path: QString) -> bool {
        let path_str = path.to_string();
        tracing::info!("Loading file: {}", path_str);

        // Basic validation
        if path_str.is_empty() {
            self.error_occurred(QString::from("File path is empty"));
            return false;
        }

        // Check if file exists
        if !Path::new(&path_str).exists() {
            self.error_occurred(QString::from("File does not exist"));
            return false;
        }

        // Check if it's a WAV file
        if !path_str.to_lowercase().ends_with(".wav") {
            self.error_occurred(QString::from("Only WAV files are supported"));
            return false;
        }

        // Update UI immediately
        self.current_file = path;
        self.current_file_changed();

        // Send load command to audio engine
        self.send_audio_command(AudioCommand::LoadFile(path_str));

        // Give audio engine a moment to process, then update state
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.update_state_from_engine();

        true
    }

    fn get_player_info(&self) -> QVariantMap {
        let mut map = QVariantMap::default();
        map.insert("version".into(), QVariant::from(QString::from("1.0.0")));
        map.insert("backend".into(), QVariant::from(QString::from("cpal")));
        map.insert("supports_hot_reload".into(), QVariant::from(true));

        if let Some(ref engine) = self.audio_engine {
            map.insert(
                "current_device".into(),
                QVariant::from(QString::from(engine.get_device_name())),
            );
            map.insert("audio_engine_active".into(), QVariant::from(true));
        } else {
            map.insert("audio_engine_active".into(), QVariant::from(false));
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
                                        // Create new audio engine with this device
                                        match AudioEngine::new(device) {
                                            Ok(engine) => {
                                                self.audio_engine = Some(engine);
                                                self.current_device = device_name;
                                                self.current_device_changed();
                                                tracing::info!(
                                                    "Successfully switched to device: {}",
                                                    device_name_str
                                                );
                                                return true;
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to create audio engine for device {}: {}", device_name_str, e);
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

    // Public helper method for periodic state updates
    pub fn update_state(&mut self) {
        self.update_state_from_engine();
    }
}
