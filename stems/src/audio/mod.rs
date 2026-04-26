//! Audio module for stem playback and device management

pub mod device_manager;
pub mod engine;
pub mod filter;
pub mod metadata;
pub mod multi_engine;
pub mod stem_discovery;
pub mod wav_loader;

pub use device_manager::{AudioDevice, DeviceManager};
pub use engine::{AudioCommand, AudioEngine, AudioEngineState};
pub use metadata::{extract_metadata, extract_metadata_from_first_file, SongMetadata};
pub use multi_engine::{MultiAudioCommand, MultiAudioEngine, MultiAudioState};
pub use stem_discovery::{StemDiscovery, StemMatch};
pub use wav_loader::{AudioFile, MappedAudioFile, WavLoader};
