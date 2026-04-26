use crate::audio::wav_loader::MappedAudioFile;
/// Python beat detection integration module
/// Handles calling Python librosa-based beat detection from Rust
use anyhow::{Context, Result};
use pyo3::prelude::*;
use pyo3::types::{PyFloat, PyList};
use pythonize::depythonize;
use serde::{Deserialize, Serialize};
use std::ffi::CString;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsistencyData {
    pub consistency: f64,
    pub std_deviation: f64,
    pub regularity: f64,
    pub mean_interval: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythonBeatResult {
    pub success: bool,
    pub tempo: f64,
    pub beat_timestamps: Vec<f64>,
    pub beat_count: usize,
    pub onset_timestamps: Vec<f64>,
    pub onset_count: usize,
    pub avg_beat_interval: f64,
    pub duration: f64,
    pub sample_rate: u32,
    pub consistency: Option<ConsistencyData>,
    pub error: Option<String>,
}

/// Call Python beat detection on a MappedAudioFile using in-memory data
pub fn detect_beats_python(audio_file: &MappedAudioFile) -> Result<PythonBeatResult> {
    Python::with_gil(|py| -> Result<PythonBeatResult> {
        tracing::debug!(
            "Running Python beat detection on audio file with {} samples",
            audio_file.sample_count
        );

        // Convert audio samples to Python list (MappedAudioFile already normalizes to [-1.0, 1.0])
        let audio_data = PyList::empty(py);

        for i in 0..audio_file.sample_count {
            let sample = audio_file.get_sample(i) as f64;
            audio_data.append(PyFloat::new(py, sample))?;
        }

        // Load Python code from external script
        let beat_detection_code = include_str!("beat_detection.py");

        // Convert to CString for PyO3
        let code_cstr = CString::new(beat_detection_code)
            .context("Failed to create CString from Python code")?;

        // Create a namespace and execute the Python code to define the function
        let locals = pyo3::types::PyDict::new(py);
        py.run(&code_cstr, None, Some(&locals))
            .context("Failed to execute Python beat detection script")?;

        // Get the detect_beats function from the Python namespace
        let detect_beats_fn = locals
            .get_item("detect_beats")
            .context("Failed to get 'detect_beats' function from Python namespace")?
            .context("Python 'detect_beats' function not found")?;

        // Call the detect_beats function with our audio data
        let py_result = detect_beats_fn
            .call1((audio_data, audio_file.spec.sample_rate))
            .context("Failed to call Python detect_beats function")?;

        // Use pythonize to deserialize the Python object directly into our Rust struct
        let beat_result: PythonBeatResult = depythonize(&py_result)
            .context("Failed to deserialize Python beat detection result")?;

        if beat_result.success {
            tracing::info!(
                "Python beat detection successful: {} beats at {:.1} BPM (consistency: {:.3})",
                beat_result.beat_count,
                beat_result.tempo,
                beat_result
                    .consistency
                    .as_ref()
                    .map(|c| c.consistency)
                    .unwrap_or(0.0)
            );
        } else {
            tracing::warn!("Python beat detection failed: {:?}", beat_result.error);
        }

        Ok(beat_result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_detection_integration() -> Result<()> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "stems=debug,info".to_string()),
            )
            .init();

        // This test requires the Python environment to be set up
        // and the test file to exist, so it's more of an integration test
        use crate::audio::wav_loader::WavLoader;

        tracing::info!("Testing Python beat detection integration...");

        // Test with a known drums file - load it into a MappedAudioFile
        let drums_file = "/Users/sam/sam-repos/stems/demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums.wav";

        if !std::path::Path::new(drums_file).exists() {
            tracing::warn!("Test drums file not found: {}", drums_file);
            return Err(anyhow::anyhow!("Test drums file not found: {}", drums_file));
        }

        let audio_file = match WavLoader::load_file_mapped(drums_file) {
            Ok(audio_file) => audio_file,
            Err(e) => {
                tracing::warn!("Failed to load test drums file: {}", e);
                return Err(e);
            }
        };

        tracing::info!(
            "Loaded audio file: {} samples, {} channels, {} Hz",
            audio_file.sample_count,
            audio_file.spec.channels,
            audio_file.spec.sample_rate
        );

        let result = match detect_beats_python(&audio_file) {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!("Python beat detection test error: {}", e);
                return Err(e);
            }
        };

        if result.success {
            tracing::info!(
                "Python beat detection test successful: {} beats at {:.1} BPM",
                result.beat_count,
                result.tempo
            );
        } else {
            tracing::warn!("Python beat detection test failed: {:?}", result.error);
        }

        Ok(())
    }
}
