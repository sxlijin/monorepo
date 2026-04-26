use anyhow::{Context, Result};
use hound::{WavReader, WavSpec};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AudioFile {
    pub path: String,
    pub spec: WavSpec,
    pub duration_seconds: f64,
    pub samples: Vec<f32>,
}

#[derive(Debug)]
pub struct MappedAudioFile {
    pub path: String,
    pub spec: WavSpec,
    pub duration_seconds: f64,
    pub sample_count: usize,
    pub data_offset: usize,
    pub mmap: Mmap,
}

pub struct WavLoader;

impl MappedAudioFile {
    pub fn get_sample(&self, index: usize) -> f32 {
        if index >= self.sample_count {
            return 0.0;
        }

        let byte_offset = self.data_offset + index * std::mem::size_of::<i16>();
        if byte_offset + 1 >= self.mmap.len() {
            return 0.0;
        }

        // Read i16 sample from memory-mapped data
        let sample_bytes = [self.mmap[byte_offset], self.mmap[byte_offset + 1]];
        let sample_i16 = i16::from_le_bytes(sample_bytes);

        // Convert to f32 in range [-1.0, 1.0]
        sample_i16 as f32 / 32768.0
    }
}

impl WavLoader {
    /// Load audio and return a mono i16 buffer at the source sample rate.
    /// - For mono input: copies samples as-is (with format conversion if needed).
    /// - For multi-channel: averages channels per frame with rounding and clamps to i16.
    pub fn load_mono_i16<P: AsRef<Path>>(path: P) -> Result<(Vec<i16>, WavSpec)> {
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();

        tracing::info!("Loading mono i16 from WAV: {}", path_str);

        let mut reader = WavReader::open(path_ref)
            .with_context(|| format!("Failed to open WAV file: {}", path_str))?;

        let spec = reader.spec();

        let channels = spec.channels as usize;
        if channels == 0 {
            return Err(anyhow::anyhow!("WAV reports zero channels"));
        }

        let mut mono: Vec<i16> = Vec::new();

        match spec.sample_format {
            hound::SampleFormat::Float => {
                // Read float samples in [-1,1]
                let mut frame: Vec<f32> = vec![0.0; channels];
                let mut iter = reader.samples::<f32>();
                loop {
                    // Fill one frame
                    let mut got = 0usize;
                    for c in 0..channels {
                        if let Some(s) = iter.next() {
                            frame[c] = s.context("Failed to read float sample")?;
                            got += 1;
                        } else {
                            break;
                        }
                    }
                    if got == 0 {
                        break;
                    }
                    if got < channels {
                        break;
                    }
                    let sum: f32 = frame.iter().take(channels).copied().sum();
                    let avg = sum / (channels as f32);
                    let scaled = (avg * 32768.0)
                        .round()
                        .clamp(i16::MIN as f32, i16::MAX as f32);
                    mono.push(scaled as i16);
                }
            }
            hound::SampleFormat::Int => {
                let bps = spec.bits_per_sample;
                if bps <= 16 {
                    // Read i16 directly
                    let mut frame: Vec<i16> = vec![0; channels];
                    let mut iter = reader.samples::<i16>();
                    loop {
                        let mut got = 0usize;
                        for c in 0..channels {
                            if let Some(s) = iter.next() {
                                frame[c] = s.context("Failed to read i16 sample")?;
                                got += 1;
                            } else {
                                break;
                            }
                        }
                        if got == 0 {
                            break;
                        }
                        if got < channels {
                            break;
                        }
                        // average with widening to avoid overflow
                        let sum: i32 = frame.iter().take(channels).map(|&v| v as i32).sum();
                        let avg =
                            (sum / channels as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                        mono.push(avg);
                    }
                } else {
                    // e.g., 24-bit -> read as i32 and scale to i16
                    let mut frame: Vec<i32> = vec![0; channels];
                    let mut iter = reader.samples::<i32>();
                    let max_val = (1i64 << (bps - 1)) as f64; // e.g., 2^23
                    loop {
                        let mut got = 0usize;
                        for c in 0..channels {
                            if let Some(s) = iter.next() {
                                frame[c] = s.context("Failed to read i32 sample")?;
                                got += 1;
                            } else {
                                break;
                            }
                        }
                        if got == 0 {
                            break;
                        }
                        if got < channels {
                            break;
                        }
                        let sum: f64 = frame
                            .iter()
                            .take(channels)
                            .map(|&v| v as f64 / max_val)
                            .sum();
                        let avg = (sum / (channels as f64)).clamp(-1.0, 1.0);
                        let s16 = (avg * 32768.0)
                            .round()
                            .clamp(i16::MIN as f64, i16::MAX as f64)
                            as i16;
                        mono.push(s16);
                    }
                }
            }
        }

        // Derive duration based on frames read
        let duration_seconds = mono.len() as f64 / (spec.sample_rate as f64);
        tracing::info!(
            "Loaded mono: {} frames, {:.2}s, {} Hz (source channels {})",
            mono.len(),
            duration_seconds,
            spec.sample_rate,
            spec.channels
        );

        Ok((mono, spec))
    }
    pub fn load_file_mapped<P: AsRef<Path>>(path: P) -> Result<MappedAudioFile> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        tracing::info!("Memory-mapping WAV file: {}", path_str);

        // Open file and create memory map
        let file =
            File::open(path).with_context(|| format!("Failed to open WAV file: {}", path_str))?;

        let mmap = unsafe { Mmap::map(&file) }
            .with_context(|| format!("Failed to memory-map file: {}", path_str))?;

        // Read WAV header using hound to get spec and data offset
        let mut reader = WavReader::open(path)
            .with_context(|| format!("Failed to read WAV header: {}", path_str))?;

        let spec = reader.spec();

        // Calculate data offset by reading until we hit sample data
        let mut temp_reader = WavReader::open(path)?;
        let _: Vec<i16> = temp_reader
            .samples::<i16>()
            .take(1)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to find sample data")?;

        // For simplicity, assume standard WAV header size (44 bytes for basic WAV)
        let data_offset = 44usize;

        let sample_count = ((mmap.len() - data_offset) / std::mem::size_of::<i16>())
            .min(spec.channels as usize * spec.sample_rate as usize * 300); // Max 5 minutes

        let duration_seconds =
            sample_count as f64 / (spec.sample_rate as f64 * spec.channels as f64);

        tracing::info!(
            "Memory-mapped WAV: {} samples, {:.2}s duration, {}Hz, {} channels, data_offset: {}",
            sample_count,
            duration_seconds,
            spec.sample_rate,
            spec.channels,
            data_offset
        );

        Ok(MappedAudioFile {
            path: path_str,
            spec,
            duration_seconds,
            sample_count,
            data_offset,
            mmap,
        })
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<AudioFile> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        tracing::info!("Loading WAV file: {}", path_str);

        let mut reader = WavReader::open(path)
            .with_context(|| format!("Failed to open WAV file: {}", path_str))?;

        let spec = reader.spec();

        // Read samples and convert to f32
        let samples: Result<Vec<f32>> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to read float samples"),
            hound::SampleFormat::Int => {
                let int_samples: Vec<i32> = reader
                    .samples::<i32>()
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to read int samples")?;

                // Convert to f32 based on bit depth
                let max_value = (1i32 << (spec.bits_per_sample - 1)) as f32;
                Ok(int_samples
                    .into_iter()
                    .map(|s| s as f32 / max_value)
                    .collect())
            }
        };

        let samples = samples?;
        let sample_count = samples.len() as u32;
        let duration_seconds =
            sample_count as f64 / (spec.sample_rate as f64 * spec.channels as f64);

        tracing::info!(
            "Loaded WAV: {} samples, {:.2}s duration, {}Hz, {} channels",
            sample_count,
            duration_seconds,
            spec.sample_rate,
            spec.channels
        );

        Ok(AudioFile {
            path: path_str,
            spec,
            duration_seconds,
            samples,
        })
    }
}
