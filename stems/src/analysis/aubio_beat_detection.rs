/// Native Rust beat detection using aubio-rs
/// Replaces Python librosa-based beat detection with direct aubio library calls
use crate::audio::wav_loader::MappedAudioFile;
use anyhow::{Context, Result};
use aubio_rs::{Onset, OnsetMode, Tempo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsistencyData {
    pub consistency: f64,
    pub std_deviation: f64,
    pub regularity: f64,
    pub mean_interval: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AubioBeatResult {
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

impl AubioBeatResult {
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            tempo: 0.0,
            beat_timestamps: Vec::new(),
            beat_count: 0,
            onset_timestamps: Vec::new(),
            onset_count: 0,
            avg_beat_interval: 0.0,
            duration: 0.0,
            sample_rate: 0,
            consistency: None,
            error: Some(message),
        }
    }
}

/// Calculate beat consistency metrics matching Python implementation
fn calculate_consistency(beat_times: &[f64]) -> ConsistencyData {
    if beat_times.len() < 2 {
        return ConsistencyData {
            consistency: 0.0,
            std_deviation: 0.0,
            regularity: 0.0,
            mean_interval: 0.0,
        };
    }

    // Calculate beat intervals (differences between consecutive beats)
    let intervals: Vec<f64> = beat_times.windows(2).map(|w| w[1] - w[0]).collect();

    let mean_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;

    // Calculate standard deviation
    let variance = intervals
        .iter()
        .map(|interval| (interval - mean_interval).powi(2))
        .sum::<f64>()
        / intervals.len() as f64;
    let std_deviation = variance.sqrt();

    // Calculate regularity as inverse of coefficient of variation
    let cv = if mean_interval > 0.0 {
        std_deviation / mean_interval
    } else {
        f64::INFINITY
    };
    let regularity = 1.0 / (1.0 + cv);

    // Overall consistency score (0-1, higher is more consistent)
    let consistency = if mean_interval > 0.0 {
        (1.0 - (std_deviation / mean_interval)).max(0.0)
    } else {
        0.0
    };

    ConsistencyData {
        consistency,
        std_deviation,
        regularity,
        mean_interval,
    }
}

/// Detect beats using aubio-rs on a MappedAudioFile
pub fn detect_beats_aubio(audio_file: &MappedAudioFile) -> Result<AubioBeatResult> {
    let sample_rate = audio_file.spec.sample_rate;
    let hop_size = 512;
    let buf_size = 1024;

    tracing::debug!(
        "Running aubio beat detection on audio file with {} samples at {} Hz",
        audio_file.sample_count,
        sample_rate
    );

    // Create tempo detector with specflux method (matches librosa default)
    let mut tempo = Tempo::new(OnsetMode::SpecFlux, buf_size, hop_size, sample_rate)
        .context("Failed to create aubio Tempo detector")?;

    // Create onset detector
    let mut onset = Onset::new(OnsetMode::SpecFlux, buf_size, hop_size, sample_rate)
        .context("Failed to create aubio Onset detector")?;

    let mut beat_samples = Vec::new();
    let mut onset_samples = Vec::new();

    // Process audio in chunks
    let mut position = 0;
    while position + buf_size <= audio_file.sample_count {
        // Extract chunk of audio samples
        let mut chunk = vec![0.0f32; buf_size];
        for i in 0..buf_size {
            chunk[i] = audio_file.get_sample(position + i);
        }

        // Detect beats in this chunk
        let mut tempo_output = vec![0.0f32; 1]; // Single output value for tempo
        let _ = tempo.do_(&chunk, &mut tempo_output[..]);

        if tempo_output[0] > 0.0 {
            let beat_sample = tempo.get_last();
            beat_samples.push(beat_sample as f64);
        }

        // Detect onsets in this chunk
        let mut onset_output = vec![0.0f32; 1]; // Single output value for onset
        let _ = onset.do_(&chunk, &mut onset_output[..]);

        if onset_output[0] > 0.0 {
            let onset_sample = onset.get_last();
            onset_samples.push(onset_sample as f64);
        }

        position += hop_size;
    }

    // Convert sample positions to timestamps (seconds)
    let beat_timestamps: Vec<f64> = beat_samples
        .into_iter()
        .map(|sample| sample / sample_rate as f64)
        .collect();

    let onset_timestamps: Vec<f64> = onset_samples
        .into_iter()
        .map(|sample| sample / sample_rate as f64)
        .collect();

    // Get tempo estimate
    let tempo_bpm = tempo.get_bpm() as f64;

    // Calculate average beat interval
    let avg_beat_interval = if beat_timestamps.len() > 1 {
        let intervals: Vec<f64> = beat_timestamps.windows(2).map(|w| w[1] - w[0]).collect();
        intervals.iter().sum::<f64>() / intervals.len() as f64
    } else {
        0.0
    };

    // Calculate consistency metrics
    let consistency = calculate_consistency(&beat_timestamps);

    let duration = audio_file.sample_count as f64 / sample_rate as f64;

    let beat_count = beat_timestamps.len();
    let onset_count = onset_timestamps.len();

    let result = AubioBeatResult {
        success: true,
        tempo: tempo_bpm,
        beat_timestamps,
        beat_count,
        onset_timestamps,
        onset_count,
        avg_beat_interval,
        duration,
        sample_rate,
        consistency: Some(consistency),
        error: None,
    };

    tracing::info!(
        "Aubio beat detection successful: {} beats at {:.1} BPM (consistency: {:.3})",
        result.beat_count,
        result.tempo,
        result
            .consistency
            .as_ref()
            .map(|c| c.consistency)
            .unwrap_or(0.0)
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::python_beat_detection::detect_beats_python;
    use crate::audio::wav_loader::WavLoader;

    #[test]
    fn test_aubio_beat_detection() -> Result<()> {
        // Initialize logging
        tracing_subscriber::fmt()
            .with_env_filter(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "stems=debug,info".to_string()),
            )
            .try_init()
            .ok(); // Ignore if already initialized

        tracing::info!("Testing aubio-rs beat detection...");

        // Test with a known drums file
        let drums_file = "/Users/sam/sam-repos/stems/demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/drums.wav";

        if !std::path::Path::new(drums_file).exists() {
            tracing::warn!("Test drums file not found: {}", drums_file);
            return Ok(());
        }

        let audio_file = match WavLoader::load_file_mapped(drums_file) {
            Ok(audio_file) => audio_file,
            Err(e) => {
                tracing::warn!("Failed to load test drums file: {}", e);
                return Ok(());
            }
        };

        tracing::info!(
            "Loaded audio file: {} samples, {} channels, {} Hz",
            audio_file.sample_count,
            audio_file.spec.channels,
            audio_file.spec.sample_rate
        );

        // Test aubio-rs implementation
        let aubio_result = detect_beats_aubio(&audio_file)?;

        if aubio_result.success {
            tracing::info!(
                "Aubio beat detection successful: {} beats at {:.1} BPM (consistency: {:.3})",
                aubio_result.beat_count,
                aubio_result.tempo,
                aubio_result
                    .consistency
                    .as_ref()
                    .map(|c| c.consistency)
                    .unwrap_or(0.0)
            );

            // Compare with Python implementation if available
            match detect_beats_python(&audio_file) {
                Ok(python_result) if python_result.success => {
                    tracing::info!(
                        "Python beat detection: {} beats at {:.1} BPM (consistency: {:.3})",
                        python_result.beat_count,
                        python_result.tempo,
                        python_result
                            .consistency
                            .as_ref()
                            .map(|c| c.consistency)
                            .unwrap_or(0.0)
                    );

                    // Compare results (allow some tolerance)
                    let tempo_diff = (aubio_result.tempo - python_result.tempo).abs();
                    let beat_count_diff =
                        (aubio_result.beat_count as i32 - python_result.beat_count as i32).abs();

                    tracing::info!(
                        "Comparison: tempo diff = {:.1} BPM, beat count diff = {}",
                        tempo_diff,
                        beat_count_diff
                    );

                    if tempo_diff > 5.0 {
                        tracing::warn!(
                            "Large tempo difference between aubio and Python: {:.1} BPM",
                            tempo_diff
                        );
                    }

                    if beat_count_diff > 10 {
                        tracing::warn!(
                            "Large beat count difference between aubio and Python: {}",
                            beat_count_diff
                        );
                    }
                }
                Ok(python_result) => {
                    tracing::warn!("Python beat detection failed: {:?}", python_result.error);
                }
                Err(e) => {
                    tracing::warn!("Python beat detection error: {}", e);
                }
            }
        } else {
            tracing::warn!("Aubio beat detection failed: {:?}", aubio_result.error);
        }

        Ok(())
    }

    #[test]
    fn test_consistency_calculation() {
        // Test with perfectly regular beats (120 BPM = 0.5 second intervals)
        let regular_beats = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
        let consistency = calculate_consistency(&regular_beats);

        assert!((consistency.mean_interval - 0.5).abs() < 0.001);
        assert!(consistency.std_deviation < 0.001);
        assert!(consistency.consistency > 0.99);
        assert!(consistency.regularity > 0.99);

        // Test with irregular beats
        let irregular_beats = vec![0.0, 0.3, 1.2, 1.4, 2.1, 2.8, 3.5];
        let consistency = calculate_consistency(&irregular_beats);

        assert!(consistency.consistency < 0.9);
        assert!(consistency.regularity < 0.9);
        assert!(consistency.std_deviation > 0.1);

        // Test with too few beats
        let few_beats = vec![0.0];
        let consistency = calculate_consistency(&few_beats);

        assert_eq!(consistency.consistency, 0.0);
        assert_eq!(consistency.std_deviation, 0.0);
        assert_eq!(consistency.regularity, 0.0);
        assert_eq!(consistency.mean_interval, 0.0);
    }
}
