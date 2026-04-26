 # Core Architecture Decisions

  1. GUI Framework Choice

  - egui - Immediate mode, simple, good for prototyping
  - tauri - Web-based UI with Rust backend
  - iced - Elm-inspired, declarative, good performance
  - slint - Declarative UI toolkit with design tools
  - gtk4-rs - Native look, platform integration

  2. Audio Backend

  - rodio - Pure Rust, simple API, good for basic playback
  - cpal - Cross-platform audio I/O, more control
  - symphonia - Advanced codec support and metadata
  - hound - WAV-specific, lightweight for your use case

  3. Application Architecture Pattern

  - MVC/MVP - Separate UI, business logic, data
  - Actor model - Message passing between components
  - ECS - Entity-component-system for complex state
  - Simple state machine - Given your 4-track stem player needs

  4. Audio Processing Architecture

  - Single-threaded - Simple but may block UI
  - Multi-threaded - Separate audio thread + UI thread
  - Channel-based - Message passing between threads
  - Callback-based - Audio engine drives timing

  5. Data Management

  - File scanning strategy - Recursive vs explicit paths
  - Metadata caching - Store duration, sample rate, etc.
  - Song/track organization - How to group stems by song
  - Playlist management - Queue, history, favorites

  6. Key Features to Decide

  - Playback controls - Play/pause/stop per track vs master
  - Mixing capabilities - Volume per track, panning, effects
  - Synchronization - Keep all 4 stems in sync
  - Seeking - Timeline scrubbing across all tracks
  - Visualization - Waveforms, meters, spectrum analyzer

  7. File Structure Assumptions

  Your demucs output has this pattern:
  htdemucs/[Song Name]/[bass.wav, drums.wav, other.wav, vocals.wav]
  - Discovery logic - How to scan and group files
  - Error handling - Missing stems, corrupted files
  - Format validation - Ensure consistent sample rates

  8. Performance Considerations

  - Memory usage - Load full files vs streaming
  - Buffer sizes - Balance latency vs CPU usage
  - Threading model - Audio callback priority
  - UI responsiveness - Non-blocking file operations

  Recommended Starting Stack

  For a focused stem player, I'd suggest:
  - GUI: iced (good balance of simplicity and capability)
  - Audio: rodio + hound (perfect for WAV playback)
  - Architecture: Multi-threaded with channels
  - Pattern: Simple state machine with audio/UI separation


# Playback Architecture Decisions

  1. Synchronization Strategy

  - Master clock approach - One audio thread drives timing for all streams
  - Sample-accurate sync - Ensure all stems start at exact same sample
  - Seek handling - How to maintain sync when jumping to timestamp
  - Buffer coordination - Keep all 4 audio buffers aligned

  2. Audio Stream Management

  - Single mixer stream - Mix 4 stems into one output (simpler)
  - Multiple parallel streams - Separate rodio sinks per stem (more control)
  - Custom audio callback - Direct cpal management for precise control
  - Stem muting/soloing - Individual track enable/disable

  Waveform Visualization Decisions

⏺ Update Todos
  ⎿  ☒ Analyze playback interface requirements
     ☒ Identify waveform visualization decisions
     ☐ Identify spectrogram visualization decisions
     ☐ Define synchronization architecture decisions

⏺ 3. Waveform Data Processing

  - Pre-computation vs real-time - Generate waveform data on load or on-demand
  - Resolution levels - Multiple zoom levels (overview + detail)
  - Peak detection algorithm - RMS, peak, or both for amplitude
  - Downsampling strategy - How to reduce 44.1kHz to screen pixels
  - Storage format - Cache waveform data as binary/JSON/in-memory

  4. Waveform Rendering

  - Canvas choice - Custom drawing vs GUI framework primitives
  - Update frequency - Real-time cursor vs periodic refresh
  - Memory usage - Full resolution vs viewport-based rendering
  - Color coding - Different colors per stem, amplitude mapping
  - Stereo handling - Show both channels or mix to mono

  Spectrogram Decisions

⏺ Update Todos
  ⎿  ☒ Analyze playback interface requirements
     ☒ Identify waveform visualization decisions
     ☒ Identify spectrogram visualization decisions
     ☐ Define synchronization architecture decisions

⏺ 5. FFT Processing

  - FFT library choice - rustfft, realfft, or fftw bindings
  - Window size - Balance frequency/time resolution (1024, 2048, 4096 samples)
  - Window function - Hann, Blackman, or Hamming for spectral leakage
  - Overlap percentage - 50-75% overlap for smooth visualization
  - Frequency range - Full spectrum vs focus on musical range (80Hz-8kHz)

  6. Spectrogram Rendering

  - Color mapping - Magnitude to color (linear, log, mel scale)
  - Real-time vs pre-computed - CPU vs memory tradeoff
  - Texture/image backend - How to efficiently render frequency data
  - Zoom capabilities - Time/frequency axis scaling
  - Multiple spectrograms - One per stem or combined view

  UI Layout & Interaction Decisions

  7. Interface Organization

  - Track layout - Horizontal stacking vs tabbed views
  - Waveform size - Fixed height vs resizable panes
  - Spectrogram placement - Below waveform, separate panel, or overlay
  - Timeline ruler - Shared timeline across all stems
  - Individual controls - Per-stem volume/mute/solo placement

  8. Click-to-Seek Implementation

  - Coordinate mapping - Screen pixels to audio samples
  - Visual feedback - Playhead cursor, selection highlighting
  - Precision handling - Snap-to-grid vs free positioning
  - Multi-track coordination - One seek affects all stems

  Performance & Caching Decisions

  9. Data Pipeline

  - Background processing - Generate visualizations while playing
  - Progressive loading - Show basic waveform first, detailed spectrogram later
  - Memory limits - Max cached visualization data per song
  - Thread allocation - Separate threads for audio, visualization, UI

  Recommended Libraries

  - FFT: rustfft (pure Rust, good performance)
  - Visualization: Custom canvas with iced::canvas or egui::plot
  - Audio analysis: symphonia for metadata + sample access
  - Rendering: GPU-accelerated textures for spectrograms

# Audio synchronization

  Most Successful Approach: Single audio thread with pull-based mixing
  - One high-priority audio callback
  - Callback requests samples from all 4 stems at current position
  - All stems advance by exactly same sample count
  - Seek operations update shared position atomically

# macos arm notes


⏺ 1. cpal (Most Battle-Tested)

  - Core Audio backend - Direct Core Audio bindings
  - Used by: Firefox, Servo, many Rust audio apps
  - ARM64: Native support, optimized callbacks
  cpal = "0.15"

  2. coreaudio-rs (Native Core Audio)

  - Direct Core Audio bindings - AudioUnit, AudioQueue APIs
  - Professional grade - Same APIs Logic Pro uses
  - ARM64: Full native support
  coreaudio-rs = "0.11"

  3. PortAudio Bindings (Cross-Platform C Library)

  - portaudio-rs - Rust bindings to PortAudio C library
  - Extremely battle-tested - Used by Audacity, many pro audio tools
  - ARM64: PortAudio has excellent macOS ARM support
  portaudio = "0.7"

  FFT/DSP Libraries (Battle-Tested Priority)

⏺ Update Todos
  ⎿  ☒ Research Rust GUI libraries for macOS ARM
     ☒ Research Rust audio libraries for macOS ARM
     ☒ Research FFT/DSP libraries for macOS ARM
     ☐ Research C/C++ library bindings for macOS ARM

⏺ 1. vDSP (Apple's Accelerate Framework)

  - accelerate-rs - Rust bindings to Apple's vDSP
  - Most battle-tested on macOS - Hardware-optimized for Apple Silicon
  - ARM64: Native NEON optimizations
  accelerate-src = "0.3"  # Links Apple's Accelerate framework

  2. FFTW Bindings (Industry Standard)

  - fftw - Rust bindings to FFTW C library
  - Most battle-tested globally - Used by MATLAB, NumPy, scientific computing
  - ARM64: FFTW 3.3.10+ has excellent Apple Silicon support
  fftw = "0.8"

  3. Intel MKL (via Science/Math Libraries)

  - intel-mkl-src - Intel Math Kernel Library
  - Extremely battle-tested - Used by professional scientific software
  - ARM64: Intel MKL now supports Apple Silicon via Rosetta + native
  intel-mkl-src = "0.8"

  C/C++ Library Bindings (Maximum Battle-Testing)

⏺ Update Todos
  ⎿  ☒ Research Rust GUI libraries for macOS ARM
     ☒ Research Rust audio libraries for macOS ARM
     ☒ Research FFT/DSP libraries for macOS ARM
     ☒ Research C/C++ library bindings for macOS ARM

⏺ 1. JUCE Framework (C++)

  - juce-rs or direct C++ integration via cc crate
  - Most battle-tested for audio apps - Used by major DAWs
  - ARM64: Full native support
  - Features: Audio I/O, GUI, DSP, plugin hosting

  2. Qt6 (C++)

  - qmetaobject or cxx-qt - Rust-Qt bindings
  - Extremely battle-tested - Desktop standard for 25+ years
  - ARM64: Full native support
  - Audio: Qt Multimedia module

  3. Dear ImGui (C++)

  - imgui-rs - Rust bindings to Dear ImGui
  - Battle-tested in games/tools - Used by many audio tools
  - ARM64: Full support
  - Performance: Immediate mode, excellent for real-time displays

  4. FMOD (C++)

  - libfmod-sys - Professional audio engine
  - Most battle-tested in games - AAA game industry standard
  - ARM64: Full native support
  - Features: 3D audio, effects, streaming

  Recommended Stack for Maximum Battle-Testing

  [dependencies]
  # GUI: Most native macOS experience
  cacao = "0.4"           # or tauri = "2.0" for web-based

  # Audio: Industry standard
  portaudio = "0.7"       # PortAudio C library bindings
  hound = "3.5"          # WAV file reading

  # FFT: Apple's optimized framework
  accelerate-src = "0.3"  # Apple vDSP/Accelerate

  # Graphics: Battle-tested immediate mode
  imgui = "0.11"         # Dear ImGui for visualizations

  Alternative Maximum Battle-Test Stack:
  # Use JUCE for everything audio + basic GUI
  cc = "1.0"             # Build JUCE C++ code
  cxx = "1.0"            # Safe C++ interop
  # + write JUCE C++ wrapper for audio engine
  # + Tauri for main GUI with JUCE audio backend

  The PortAudio + Apple Accelerate + Dear ImGui combination gives you the most battle-tested foundation, with each library having 15-25 years of production use.