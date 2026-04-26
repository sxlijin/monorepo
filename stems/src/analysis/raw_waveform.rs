// Raw waveform data container used for rendering from mono i16 samples

#[derive(Debug, Clone)]
pub struct LodLevel {
    pub samples_per_bin: usize,
    pub min: Vec<i16>,
    pub max: Vec<i16>,
}

impl LodLevel {
    pub fn len(&self) -> usize {
        self.min.len().min(self.max.len())
    }
}

#[derive(Debug, Clone)]
pub struct RawWaveformData {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
    pub mono: Vec<i16>,
    pub beat_timestamps: Vec<f64>,
    pub tempo: Option<f64>,
    // Multi-resolution peaks pyramid for zoomed-out rendering
    pub lod: Vec<LodLevel>,
}

impl RawWaveformData {
    pub fn frame_count(&self) -> usize {
        self.mono.len()
    }

    /// Build a min/max LOD pyramid from mono samples.
    /// Starts at `start_bin` samples per bin and doubles up to `max_bin` or until bin >= mono.len().
    pub fn build_lod_pyramid(mono: &[i16], start_bin: usize, max_bin: usize) -> Vec<LodLevel> {
        let n = mono.len();
        if n == 0 || start_bin == 0 {
            return Vec::new();
        }
        let mut levels = Vec::new();
        let mut bin = start_bin;
        while bin <= max_bin && bin < n.saturating_add(1) {
            let bins = (n + bin - 1) / bin; // ceil
            let mut mins = Vec::with_capacity(bins);
            let mut maxs = Vec::with_capacity(bins);
            let mut start = 0;
            while start < n {
                let end = (start + bin).min(n);
                let mut mn = i16::MAX;
                let mut mx = i16::MIN;
                // simple linear reduction; can be SIMD-optimized later
                for &v in &mono[start..end] {
                    if v < mn {
                        mn = v;
                    }
                    if v > mx {
                        mx = v;
                    }
                }
                mins.push(mn);
                maxs.push(mx);
                start += bin;
            }
            levels.push(LodLevel {
                samples_per_bin: bin,
                min: mins,
                max: maxs,
            });
            bin = bin.saturating_mul(2);
            if bin == 0 {
                break;
            }
        }
        levels
    }
}
