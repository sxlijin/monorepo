# Beat Analysis Plan

## Overview
Add periodic beat markers to the waveform visualization by using Python's librosa library for beat detection and integrating it into the existing Rust/Qt music player.

## Implementation Strategy

### 1. Python Beat Detection Module
- Create a Python script using librosa for beat tracking:
  - `librosa.beat.beat_track()` to detect beat timestamps
  - `librosa.load()` to load audio files
  - Return beat positions as timestamps (seconds)

### 2. Rust-Python Integration  
- Use existing pyo3 integration (already configured in Cargo.toml)
- Create Rust wrapper functions to call Python beat detection
- Handle data conversion between Python and Rust types

### 3. Waveform Data Model Extension
- Extend `WaveformData` struct to include beat markers
- Add `beat_timestamps: Vec<f64>` field for beat positions
- Update `WaveformAnalyzer` to generate both peaks and beats

### 4. QML Visualization Updates
- Modify waveform Canvas rendering to draw beat markers
- Add vertical lines at beat positions overlaid on waveform
- Make beat markers visually distinct (different color/style)

### 5. Bridge Integration
- Update `MultiBridge` to expose beat data to QML
- Add methods for accessing beat information per file
- Ensure beat detection runs alongside waveform generation

## Key Files to Modify
- `src/analysis/waveform.rs` - Add beat detection integration
- `src/player/multi_bridge.rs` - Expose beat data to QML  
- `qml/multi_player.qml` - Update Canvas rendering for beat markers
- New: Python beat detection module

## Technical Approach
- Use librosa's proven beat tracking algorithms
- Integrate seamlessly with existing waveform pipeline
- Maintain performance by caching beat data
- Keep UI responsive during beat analysis