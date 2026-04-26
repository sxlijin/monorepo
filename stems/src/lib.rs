//! Stems - Multi-track audio player and analysis library
//!
//! This library provides functionality for:
//! - Multi-file audio playback with individual stem controls
//! - Real-time waveform analysis and visualization
//! - Audio device management and streaming
//! - Qt/QML integration for UI components

pub mod analysis;
pub mod audio;
pub mod constants;
pub mod player;
pub mod ui;
pub mod utils;
pub mod waveform_registry;

// Re-export commonly used types for convenience
pub use analysis::RawWaveformData;
pub use audio::{DeviceManager, MultiAudioCommand, MultiAudioEngine};
pub use player::{LatencyReproBridge, MultiBridge};
pub use ui::{LatencyWaveformComponent, WaveformComponent, WaveformView};

// Re-export result type for consistency
pub use anyhow::Result;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default number of samples per waveform pixel for analysis
pub const DEFAULT_SAMPLES_PER_PIXEL: usize = 100;
