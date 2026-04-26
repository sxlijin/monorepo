# Phase 3: Waveform Generation Implementation Plan

## Phase 3A: Waveform Generation Foundation (Start Here)

### Step 1: Waveform Analysis Binary
Before integrating with the UI, we need a standalone tool to:
- **Test waveform generation algorithms** on our existing stem files
- **Validate peak detection accuracy** across different zoom levels
- **Benchmark performance** with memory-mapped file access
- **Debug and iterate** on waveform data structures

**Implementation:**
- Add `waveform-analyzer` binary target to Cargo.toml
- Create `src/waveform_analyzer_main.rs` with CLI interface
- Target our test files: `/Users/sam/sam-repos/stems/demucs-sandbox/separated/htdemucs/Alannah Myles - Black Velvet 0/`
- Output: JSON/binary waveform data for validation

### Step 2: Core Waveform Generation
- **Peak detection algorithm**: Sliding window min/max extraction
- **Multi-resolution downsampling**: Generate multiple detail levels (1:1, 1:10, 1:100, 1:1000 ratios)
- **Memory-efficient processing**: Stream processing to handle large files
- **Format-agnostic design**: Work with our existing MappedAudioFile system

### Step 3: Test and Validate
- **Test with our stem files**: vocals.wav, drums.wav, other.wav, bass.wav
- **Validate accuracy**: Compare peaks with audio editor waveforms
- **Performance benchmarks**: Memory usage and processing time
- **Output formats**: Design data structures for UI integration

## Overview
Implement real-time waveform visualization using Rust-based QQuickPaintedItem components following Gyroflow's proven patterns. This builds on the completed Phase 2 foundation with volume state management in Rust.

## Core Implementation Tasks

### 1. Waveform Data Generation Backend (`src/analysis/`)
- **Create `waveform.rs`**: Downsample audio data to create waveform points for visualization
- **Add analysis cache system**: Store pre-computed waveform data to avoid recalculation
- **Peak detection algorithm**: Efficient min/max calculation for waveform display at different zoom levels
- **Multi-resolution support**: Generate waveform data at multiple detail levels (overview, detailed, zoomed)

### 2. Rust QML Waveform Component (`src/ui/`)
- **Create `WaveformView.rs`**: Custom QQuickPaintedItem for high-performance waveform rendering
- **Implement paint() method**: Use QPainter for hardware-accelerated drawing
- **Real-time position indicator**: Visual playback cursor synchronized with audio engine
- **Multi-file waveform support**: Render 4 separate waveforms (vocals, drums, other, bass)

### 3. Waveform Analysis Integration
- **Connect to MultiBridge**: Expose waveform generation methods to QML
- **Background processing**: Generate waveforms on file load without blocking UI
- **Volume-aware rendering**: Use Rust volume state to adjust waveform opacity/color
- **Progress tracking**: Loading indicators during waveform analysis

### 4. QML Waveform UI Component (`qml/components/`)
- **Create `WaveformDisplay.qml`**: Container for Rust waveform components
- **Timeline integration**: Click-to-seek functionality on waveform
- **Zoom controls**: Mouse wheel zoom, pan gestures
- **Multi-stem layout**: Stacked waveforms with individual file controls

### 5. Mouse Interaction System
- **Click-to-seek**: Convert mouse position to audio timestamp and seek
- **Zoom functionality**: Mouse wheel for timeline zoom in/out
- **Pan support**: Drag to scroll timeline when zoomed
- **Hover feedback**: Show timestamp under mouse cursor

### 6. Performance Optimization
- **Efficient rendering**: Only redraw visible waveform sections
- **Frame rate management**: 60fps UI updates without audio thread interference  
- **Memory management**: Efficient waveform data storage and cleanup
- **Lazy loading**: Generate waveform data on-demand

## Technical Architecture

### Waveform Data Flow
```
Audio Files → Memory-mapped access → Peak detection → Multi-resolution cache → QQuickPaintedItem → GPU rendering
```

### Key Dependencies to Add
- No new major dependencies needed (QPainter already available via qmetaobject)
- Utilize existing `hound` for WAV sample access
- Leverage existing memory-mapped file system for performance

### Integration Points
- **MultiBridge**: Expose waveform generation and state
- **MultiAudioEngine**: Provide sample data access for analysis
- **Multi_player.qml**: Add waveform display section
- **StemControls**: Visual feedback with waveform opacity

## Deliverables
1. **Real-time waveform visualization** for all 4 stems
2. **Interactive timeline** with click-to-seek and zoom
3. **Performance-optimized rendering** at 60fps
4. **Visual playback position indicator** synchronized with audio
5. **Multi-resolution waveform support** for detailed editing

## Success Criteria
- Waveforms render at 60fps during playback
- Click-to-seek accuracy within 1 sample
- Zoom levels from full song overview to sample-level detail
- No audio dropouts during waveform rendering
- Memory usage under 100MB for waveform data per 4-minute song

## Implementation Steps
1. Write this plan to `phase3-plan.md` ✅
2. **Create waveform analysis binary**: Add standalone binary target for testing waveform generation on our test .wav files
3. **Implement core waveform generation logic**: Peak detection and downsampling algorithms
4. **Test waveform generation**: Validate output with our existing stem files (vocals.wav, drums.wav, etc.)
5. Create the `src/analysis/` module structure
6. Create Rust QML waveform component
7. Build QML UI integration
8. Add mouse interaction system
9. Optimize performance and test