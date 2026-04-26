# Phase 3A: Waveform Generation Foundation - Detailed Plan

## Overview
Before building the UI integration, we need a solid foundation for waveform generation. This phase focuses on creating a standalone waveform analyzer binary to test, validate, and benchmark our waveform generation algorithms.

## Waveform Generation Strategies

### 1. Peak Detection Approach (Recommended)
**Concept**: Extract min/max peaks from sliding windows of audio samples
**Advantages**: 
- Fast processing and rendering
- Compact data representation
- Works well for visual waveforms at any zoom level
- Industry standard approach (used by DAWs like Pro Tools, Logic Pro)

**Implementation**:
```rust
struct WaveformPeak {
    min: f32,
    max: f32,
    rms: f32,  // Optional: for amplitude-based coloring
}

// For each window of N samples, find min/max
for chunk in audio_samples.chunks(samples_per_pixel) {
    let min = chunk.iter().fold(0.0f32, |acc, &x| acc.min(x));
    let max = chunk.iter().fold(0.0f32, |acc, &x| acc.max(x));
    peaks.push(WaveformPeak { min, max, rms: calculate_rms(chunk) });
}
```

### 2. RMS (Root Mean Square) Approach
**Concept**: Calculate average energy/loudness for each window
**Advantages**:
- Better represents perceived loudness
- Smoother visual appearance
- Good for level meters and dynamics visualization

**Disadvantages**:
- Less detail than peak detection
- Doesn't show transients as clearly

### 3. Spectral Analysis Approach
**Concept**: Use FFT to analyze frequency content over time
**Advantages**:
- Can generate spectrograms
- Frequency-specific waveforms
- More musical information

**Disadvantages**:
- Much more computationally expensive
- Complex data structures
- Overkill for basic waveform display

### 4. Hybrid Approach
**Concept**: Combine peak detection with RMS and optional spectral data
**Advantages**:
- Best of all worlds
- Can choose visualization based on zoom level
- Professional DAW-quality

**Implementation Strategy**:
- Use peak detection for zoomed-out views
- Add RMS for dynamics visualization
- Optional FFT for spectrogram mode

## Available Rust Crates/Libraries

### Dedicated Audio Visualization Crates ⭐

#### 1. `audio-visualizer` (Highly Relevant)
- **Purpose**: Simple audio visualization library for waveforms and spectrum
- **Pros**: Specifically designed for our use case, PNG output, developer-friendly
- **GitHub**: https://github.com/phip1611/audio-visualizer
- **Features**: Waveform display, frequency spectrum, basic PNG output
- **Use Case**: Exactly what we need for Phase 3A validation
```toml
audio-visualizer = "0.3"
```

#### 2. `sonogram` (Spectrogram Focus) ⭐
- **Purpose**: Create spectrograms from WAV files or waveform data
- **Pros**: Mature crate, CLI + library, PNG/CSV output, direct WAV support
- **GitHub**: https://github.com/psiphi75/sonogram
- **Features**: .wav → .png spectrogram, configurable window sizes, color gradients
- **Use Case**: Future spectrogram features, validation against known good spectrograms
```toml
sonogram = "0.7"
```

### Audio Processing Crates

#### 3. `hound` (Already in use) ⭐
- **Purpose**: WAV file reading/writing
- **Pros**: Simple, reliable, no dependencies
- **Cons**: WAV-only, no advanced processing
- **Status**: Already integrated with our MappedAudioFile system

#### 4. `dasp` (Digital Audio Signal Processing)
- **Purpose**: General audio processing, resampling, filtering
- **Pros**: Comprehensive, well-designed API, good performance
- **Cons**: Might be overkill for simple peak detection
- **Use Case**: Resampling, filtering, format conversion
```toml
dasp = "0.11"
```

#### 5. `rustfft` (For spectrograms)
- **Purpose**: Fast Fourier Transform
- **Pros**: Pure Rust, fast, used by sonogram crate
- **Cons**: Not needed for basic waveforms
- **Use Case**: Future spectrogram features, or use `sonogram` instead
```toml
rustfft = "6.1"
```

### Mathematical/Processing Crates

#### 1. `rayon` (Already in dependencies)
- **Purpose**: Data parallelism
- **Pros**: Easy parallel processing, great for large files
- **Use Case**: Parallel waveform generation across multiple files
```rust
use rayon::prelude::*;
chunks.par_iter().map(|chunk| process_chunk(chunk)).collect()
```

#### 2. `ndarray`
- **Purpose**: N-dimensional arrays
- **Pros**: Efficient array operations, scientific computing
- **Cons**: Adds complexity, might be overkill
- **Use Case**: When dealing with complex multi-dimensional waveform data

#### 3. `itertools`
- **Purpose**: Iterator extensions
- **Pros**: Convenient chunking, windowing operations
- **Use Case**: Clean audio sample processing
```toml
itertools = "0.12"
```

### Serialization Crates

#### 1. `serde` + `serde_json`
- **Purpose**: Serialization for saving/loading waveform data
- **Pros**: Human-readable, debuggable
- **Cons**: Larger file sizes
- **Use Case**: Development and debugging
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### 2. `bincode`
- **Purpose**: Binary serialization
- **Pros**: Compact, fast
- **Cons**: Not human-readable
- **Use Case**: Production waveform caching
```toml
bincode = "1.3"
```

## Recommended Tech Stack for Phase 3A

### Option A: Use Existing Visualization Crates (Recommended) ⭐
```toml
# Already have these:
hound = "3.5"
memmap2 = "0.9"
anyhow = "1.0"
rayon = "1.7"

# Add for Phase 3A:
audio-visualizer = "0.3"  # For waveform validation and comparison
sonogram = "0.7"          # For spectrogram generation and validation
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Option B: Build from Scratch (Learning/Control)
```toml
# Already have these:
hound = "3.5"
memmap2 = "0.9"
anyhow = "1.0"
rayon = "1.7"

# Add for Phase 3A:
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
itertools = "0.12"
rustfft = "6.1"  # If we want spectrograms
```

### Why Option A (Existing Crates)?
1. **`audio-visualizer`**: Perfect for our Phase 3A validation - can generate PNG waveforms to compare
2. **`sonogram`**: Mature, tested spectrogram generation - gives us a reference implementation
3. **Faster development**: Don't reinvent the wheel for basic visualization
4. **Validation**: Compare our custom algorithms against proven implementations
5. **Learning**: Study existing implementations before building our own

### Why Option B (From Scratch)?
1. **Full control**: Custom data structures optimized for our specific use case
2. **Integration**: Direct integration with our MappedAudioFile system
3. **Performance**: Optimized specifically for real-time UI updates
4. **Learning**: Better understanding of algorithms

### Hybrid Approach (Best of Both) ⭐⭐
```toml
# Use existing crates for validation and reference
audio-visualizer = "0.3"
sonogram = "0.7"

# Build our own optimized implementation
itertools = "0.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**Strategy**: 
1. Use `audio-visualizer` to generate reference waveform PNGs from our test files
2. Build our own peak detection algorithm optimized for our use case
3. Compare our output against the reference images
4. Use `sonogram` for future spectrogram features

## Phase 3A Implementation Strategy

### Step 1: Basic Peak Detection
```rust
// Start simple with our existing MappedAudioFile
pub fn generate_peaks(audio_file: &MappedAudioFile, samples_per_pixel: usize) -> Vec<WaveformPeak> {
    let mut peaks = Vec::new();
    
    for i in (0..audio_file.sample_count).step_by(samples_per_pixel) {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        
        for j in 0..samples_per_pixel {
            if i + j >= audio_file.sample_count { break; }
            let sample = audio_file.get_sample(i + j);
            min = min.min(sample);
            max = max.max(sample);
        }
        
        peaks.push(WaveformPeak { min, max, rms: 0.0 });
    }
    
    peaks
}
```

### Step 2: Multi-Resolution Support
Generate multiple zoom levels:
- **Overview**: 1000+ samples per pixel (full song view)
- **Medium**: 100 samples per pixel (phrase-level)
- **Detailed**: 10 samples per pixel (note-level)
- **Sample**: 1 sample per pixel (maximum zoom)

### Step 3: CLI Interface
```bash
# Basic usage
cargo run --bin waveform-analyzer -- vocals.wav

# Multiple files
cargo run --bin waveform-analyzer -- vocals.wav drums.wav other.wav bass.wav

# Custom resolution
cargo run --bin waveform-analyzer -- --samples-per-pixel 50 vocals.wav

# Output to file
cargo run --bin waveform-analyzer -- --output waveforms.json vocals.wav
```

### Step 4: Validation
Compare our generated waveforms with:
- **Audacity** waveform export
- **Logic Pro** waveform display
- **Pro Tools** waveform view

## Success Criteria for Phase 3A
1. **Accurate peak detection**: Matches professional audio software
2. **Performance**: Process 4-minute stereo WAV in <1 second
3. **Memory efficiency**: <100MB memory usage for processing
4. **Multi-resolution**: Generate 4+ zoom levels efficiently
5. **Serialization**: Save/load waveform data for caching

## Next Phase Integration
Once Phase 3A is complete, we'll have:
- Proven waveform generation algorithms
- Performance benchmarks
- Data structures ready for UI integration
- Validated output against our test files

This foundation will make the UI integration (Phase 3B) much more straightforward and reliable.