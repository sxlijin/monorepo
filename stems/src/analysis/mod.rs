pub mod aubio_beat_detection;
pub mod beat_data;
pub mod python_beat_detection;
pub mod python_env;
pub mod python_separation;
pub mod raw_waveform;
pub mod spectral;

pub use aubio_beat_detection::{detect_beats_aubio, AubioBeatResult};
pub use beat_data::BeatData;
pub use python_beat_detection::{detect_beats_python, PythonBeatResult};
pub use python_separation::{separate_stems, SeparationResult};
pub use raw_waveform::RawWaveformData;
pub use spectral::{SpectralAnalyzer, SpectralData, FREQUENCY_BANDS};
