# Stems Player Implementation Plan

## Table of Contents
1. [Project Overview](#project-overview)
2. [Requirements](#requirements)
3. [Architecture](#architecture)
4. [Technology Stack](#technology-stack)
5. [Project Structure](#project-structure)
6. [Implementation Roadmap](#implementation-roadmap)
7. [Development Workflow](#development-workflow)
8. [Quality Assurance](#quality-assurance)
9. [Success Metrics](#success-metrics)

---

## Project Overview

### Vision
A high-performance Rust desktop application for playing multi-stem audio files with real-time waveform and spectrogram visualization. Designed specifically for demucs-separated audio stems (bass, drums, other, vocals) with professional-grade synchronization and visualization capabilities.

Target user: musicians, dancers, content creators interested in doing stem analysis on a per-song basis. Having a performant, simple
interface is a top priority.

---

## Requirements

### 1. Core Audio Functionality
- **Multi-stem playback** - Simultaneous playback of 4 audio stems with perfect synchronization
- **Transport controls** - Play, pause, stop, seek with sample-accurate timing
- **Individual stem control** - Volume, mute, solo per stem
- **Click-to-seek** - Timeline scrubbing across all stems simultaneously

### 2. File Management & Navigation
- **File browser** - Browse and select stem directories from filesystem
- **Recent files list** - Quick access to recently played stem collections
- **File metadata display** - Song title, duration, sample rate info
- **Directory scanning** - Automatic detection of demucs stem structure

### 3. Audio System Management
- **Audio device selection** - Choose different output devices (speakers, headphones, interfaces)
- **Sample rate handling** - Handle different audio formats gracefully
- **Audio buffer size control** - User-adjustable latency vs stability trade-off
- **Master volume control** - Overall output volume separate from stem mixing

### 4. User Interface & Interaction
- **Time display** - Current position, remaining time, total duration
- **Progress indicator** - Visual progress bar/timeline
- **Loading states** - Progress indication for file loading and analysis
- **Error notifications** - User-friendly error messages and recovery options
- **Keyboard shortcuts** - Space for play/pause, arrow keys for seeking, etc.
- **Mouse wheel support** - Volume adjustment, timeline scrolling
- **Context menus** - Right-click functionality for common actions
- **Drag & drop** - Drop stem directories onto application to open

### 5. Visualization Features
- **Waveform display** - Real-time waveform visualization for each stem
- **Spectrogram analysis** - Optional frequency domain visualization
- **Performance** - 60fps UI updates without affecting audio performance
- **Zoom capabilities** - Timeline zoom for detailed editing

### 6. Technical Requirements
- **Low latency** - Professional audio latency standards (<10ms)
- **Sample accuracy** - Frame-perfect synchronization between stems
- **File format support** - WAV files (16-bit, 44.1kHz stereo initially)
- **Memory efficiency** - Handle large audio files without excessive RAM usage
- **Hot reload** - Fast UI iteration during development

### 7. Platform Requirements
- **Primary target** - macOS ARM64 (Apple Silicon)
- **Native integration** - Platform-appropriate look, feel, and performance
- **Hardware acceleration** - GPU-accelerated graphics where beneficial

---

## Architecture

### Design Philosophy
Follow Gyroflow's proven architecture using qmetaobject-rs with QML frontend for optimal balance of performance, development velocity, and native platform integration.

### Core Architectural Decisions

#### GUI Framework: qmetaobject-rs + QML
**Decision**: Use qmetaobject-rs for Rust ↔ QML bridge with declarative QML frontend.

**Rationale**:
- **Battle-tested** - Gyroflow demonstrates this stack works for complex real-time media apps
- **Performance** - Zero-copy GPU rendering, hardware acceleration support
- **Development experience** - QML hot reload for rapid UI iteration
- **Native integration** - Qt provides excellent macOS platform integration
- **Minimal C++** - Mostly pure Rust with declarative QML UI

#### Audio Backend: cpal + hound
**Decision**: Use cpal for audio I/O with hound for WAV file reading.

**Rationale**:
- **cpal** - Direct Core Audio integration, proven low-latency performance
- **hound** - Lightweight, reliable WAV parsing perfect for our use case
- **Pure Rust** - Avoid C++ audio library complexity for initial implementation
- **Extensible** - Can add more codecs/formats later if needed

#### Threading Architecture: Multi-threaded with Message Passing
**Decision**: Separate threads for audio processing, UI, and file I/O with channel-based communication.

**Architecture Diagram**:
```
UI Thread (QML)           Audio Thread (cpal)         File Thread
     │                         │                         │
     ├─ qmetaobject-rs ────────┼─ mpsc channels ─────────┤
     │                         │                         │
     ├─ Transport controls     ├─ Sample mixing          ├─ WAV loading
     ├─ Waveform rendering     ├─ Synchronization        ├─ Metadata parsing
     └─ User interactions      └─ Device management      └─ Background analysis
```

#### Synchronization Strategy: Master Clock
**Decision**: Single audio thread drives timing, all other components follow.

**Implementation**:
- Audio callback maintains authoritative playback position
- UI polling via QML Timer for position updates (60fps)
- Seek operations update atomic position counter
- All 4 stems advance by identical sample counts

---

## Technology Stack

### Core Dependencies
```toml
[dependencies]
qmetaobject = "0.2"           # Rust ↔ QML bridge
cpal = "0.15"                 # Cross-platform audio I/O
hound = "3.5"                 # WAV file reading
rustfft = "6.1"               # FFT for spectrograms
```

### Audio Processing Libraries
```toml
rubato = "0.14"               # Sample rate conversion if needed
dasp = "0.11"                 # Digital audio signal processing
```

### Performance & Utilities
```toml
rayon = "1.7"                 # Parallel processing for analysis
parking_lot = "0.12"          # Fast synchronization primitives
anyhow = "1.0"                # Error handling
tracing = "0.1"               # Logging and diagnostics
```

### Build System
- **cargo** - Primary build system
- **Qt integration** - qmetaobject build scripts handle Qt dependencies
- **macOS bundling** - cargo-bundle for .app creation

---

## Project Structure

```
src/
├── main.rs                   # Application entry point, QML engine setup
├── audio/
│   ├── mod.rs               # Audio module exports
│   ├── engine.rs            # Core audio engine, cpal integration
│   ├── mixer.rs             # Multi-stem mixing logic
│   ├── synchronizer.rs      # Sample-accurate synchronization
│   └── loader.rs            # WAV file loading and parsing
├── analysis/
│   ├── mod.rs               # Analysis module exports
│   ├── waveform.rs          # Waveform data generation
│   ├── spectrogram.rs       # FFT-based spectrogram analysis
│   └── cache.rs             # Analysis result caching
├── player/
│   ├── mod.rs               # Player module exports
│   ├── controller.rs        # Transport control logic
│   ├── state.rs             # Application state management
│   └── bridge.rs            # QML ↔ Rust bridge implementation
├── library/
│   ├── mod.rs               # Library module exports
│   ├── browser.rs           # File browser and navigation
│   ├── recent.rs            # Recent files management
│   └── metadata.rs          # Audio file metadata extraction
├── devices/
│   ├── mod.rs               # Device module exports
│   ├── manager.rs           # Audio device enumeration and selection
│   └── settings.rs          # Audio device configuration
└── utils/
    ├── mod.rs               # Utility module exports
    ├── file_scanner.rs      # Demucs directory scanning
    └── config.rs            # Application configuration

qml/
├── main.qml                 # Main application window
├── components/
│   ├── TransportControls.qml # Play/pause/seek controls
│   ├── StemMixer.qml        # Individual stem controls
│   ├── WaveformView.qml     # Waveform visualization component
│   ├── Timeline.qml         # Seekable timeline component
│   ├── FileBrowser.qml      # File and directory browser
│   ├── DeviceSelector.qml   # Audio device selection
│   └── TimeDisplay.qml      # Playback time and duration display
└── styles/
    └── MacOSStyle.qml       # Platform-specific styling

resources/
├── icons/                   # Application icons and UI assets
└── examples/               # Example stem files for testing

docs/
├── user-manual.md          # End-user documentation
├── api-reference.md        # Developer API documentation
└── keyboard-shortcuts.md   # Keyboard shortcuts reference
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
**Goal**: Basic playback infrastructure with essential UI

**Core Tasks**:
- Project setup (Cargo workspace, qmetaobject integration)
- Audio device management (cpal device enumeration and selection)
- Basic audio engine (device setup, single WAV playback)
- File browser (basic file selection and loading)
- Threading architecture (separate audio/UI threads with message passing)

**UI Tasks**:
- QML interface (play/pause button, volume slider, time display)
- Keyboard shortcuts (space for play/pause, basic navigation)
- Device selector component

**Deliverable**: Single-file audio player with device selection and basic controls

### Phase 2: Multi-Stem Core (Week 3-4)
**Goal**: Synchronized 4-stem playback with file management

**Audio Tasks**:
- Stem detection (scan demucs directory structure)
- Multi-stream mixing (combine 4 WAV streams in real-time)
- Synchronization (sample-accurate timing across all stems)
- Individual controls (per-stem volume, mute, solo)
- Seek implementation (click-to-seek across all streams)

**UI Tasks**:
- Recent files tracking and display
- Drag & drop support (stem directories onto application)
- Enhanced file browser with metadata display

**Deliverable**: Full 4-stem player with individual stem control and file management

### Phase 3: Visualization (Week 5-6)
**Goal**: Real-time waveform display with interaction

**Visualization Tasks**:
- Waveform generation (downsample audio data for display)
- QML Canvas integration (custom drawing in QML)
- Real-time updates (playback position indicator)
- Multi-resolution support (zoom levels for detailed view)

**Interaction Tasks**:
- Mouse interactions (wheel zoom, click-to-seek)
- Loading indicators (progress display during analysis)
- Performance optimization (60fps without audio dropouts)

**Deliverable**: Player with real-time waveform visualization and mouse controls

### Phase 4: Advanced Features (Week 7-8)
**Goal**: Professional-grade features and polish

**Analysis Tasks**:
- Spectrogram analysis (FFT-based frequency visualization)
- Caching system (store analysis results for faster loading)
- Playlist support (navigate between multiple songs)

**UX Tasks**:
- Advanced keyboard shortcuts (professional workflow shortcuts)
- Context menus (right-click functionality for common actions)
- Error handling (user-friendly error messages and recovery)
- Audio buffer configuration (user-adjustable latency settings)

**Deliverable**: Feature-complete stem player application with advanced UI

### Phase 5: Production Polish (Week 9-10)
**Goal**: Production-ready application

**Quality Tasks**:
- Performance profiling (identify and fix bottlenecks)
- Comprehensive error handling (robust error recovery and user feedback)
- UI polish (animations, responsiveness, visual design)
- Testing suite (unit tests, integration tests, user testing)

**Documentation Tasks**:
- User manual and keyboard shortcuts reference
- API documentation for future extensibility
- Build and deployment documentation

**Deliverable**: Polished, production-ready application

---

## Development Workflow

### Environment Setup
```bash
# 1. Install Qt development dependencies
brew install qt@6

# 2. Configure Rust environment
export QT_DIR=$(brew --prefix qt@6)
export PATH="$QT_DIR/bin:$PATH"

# 3. Clone and setup project
cd stems/
cargo build

# 4. Enable hot reload for development
export STEMS_LIVE_RELOAD=1
cargo run
```

### Development Iteration Process
1. **QML hot reload** - Edit .qml files, see changes instantly
2. **Rust compilation** - `cargo check` for fast feedback
3. **Audio testing** - Test with known good stems files
4. **Performance monitoring** - Audio dropout detection, frame rate monitoring

### Git Workflow
- **Feature branches** - Separate branch for each phase/feature
- **Regular commits** - Commit working increments frequently
- **PR reviews** - Self-review before merging to main
- **Release tags** - Tag each phase completion

---

## Quality Assurance

### Testing Strategy
- **Unit tests** - Audio processing, file parsing, state management
- **Integration tests** - Full playback scenarios, synchronization accuracy
- **Performance tests** - Memory usage, CPU usage, audio latency measurements
- **Manual testing** - User workflow scenarios, edge cases, usability

### Performance Monitoring
- **Audio latency tracking** - Measure and log audio callback timing
- **Memory profiling** - Monitor memory usage patterns and leaks
- **CPU usage analysis** - Identify bottlenecks in real-time processing
- **UI responsiveness** - Frame rate monitoring during heavy operations

### Risk Mitigation

#### Technical Risks
1. **Audio latency issues** - Mitigate with cpal expertise, Core Audio optimization
2. **Qt/Rust integration complexity** - Follow Gyroflow patterns, start simple
3. **Performance bottlenecks** - Profile early, optimize critical paths
4. **Synchronization problems** - Implement atomic position tracking, test thoroughly

#### Development Risks
1. **Scope creep** - Stick to defined phases, resist feature additions
2. **Qt learning curve** - Allocate time for QML learning, use documentation
3. **Platform-specific issues** - Test on target hardware early and often
4. **Dependency updates** - Pin versions, test updates carefully

---

## Success Metrics

### Performance Targets
- **Audio latency** - <10ms round-trip latency
- **UI responsiveness** - 60fps during playback and seeking
- **Memory usage** - <500MB for typical 4-minute song with visualizations
- **Startup time** - <2 seconds from launch to ready

### Quality Targets
- **Synchronization accuracy** - <1 sample drift between stems
- **File format support** - 100% compatibility with demucs WAV output
- **Error handling** - Graceful handling of corrupted/missing files
- **User experience** - Intuitive interface requiring no manual

### Milestone Criteria
Each phase must meet its deliverable criteria before proceeding to the next phase. This ensures a solid foundation and prevents technical debt accumulation.

---

This implementation plan provides a structured, well-organized approach to building a professional-grade stem player application using proven technologies and established patterns from successful projects like Gyroflow.