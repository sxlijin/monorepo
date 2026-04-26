/// Beat data structures
/// This module provides beat data structures used throughout the application.
/// Beat detection is now performed live using aubio-rs.

#[derive(Debug, Clone)]
pub struct BeatData {
    pub tempo: f64,
    pub duration: f64,
    pub beat_timestamps: Vec<f64>,
    pub consistency: f64,
    pub regularity: f64,
}

impl BeatData {
    pub fn new(
        tempo: f64,
        duration: f64,
        beat_timestamps: Vec<f64>,
        consistency: f64,
        regularity: f64,
    ) -> Self {
        Self {
            tempo,
            duration,
            beat_timestamps,
            consistency,
            regularity,
        }
    }
}

// Note: Hardcoded beat data has been removed.
// All beat detection is now performed live using aubio-rs.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_data_creation() {
        let beat_data = BeatData::new(
            120.0,                    // tempo
            180.0,                    // duration
            vec![0.5, 1.0, 1.5, 2.0], // beat_timestamps
            0.95,                     // consistency
            0.98,                     // regularity
        );

        assert_eq!(beat_data.tempo, 120.0);
        assert_eq!(beat_data.duration, 180.0);
        assert_eq!(beat_data.beat_timestamps.len(), 4);
        assert_eq!(beat_data.consistency, 0.95);
        assert_eq!(beat_data.regularity, 0.98);
    }
}
