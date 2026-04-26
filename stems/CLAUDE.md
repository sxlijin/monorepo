We're implementing a custom music player that can play stems simultaneously.

## Vision

A high-performance Rust desktop application for playing multi-stem audio files
with real-time waveform and spectrogram visualization. Designed specifically for
demucs-separated audio stems (bass, drums, other, vocals) with
professional-grade synchronization and visualization capabilities.

Target user: musicians, dancers, content creators interested in doing stem
analysis on a per-song basis. Having a performant, simple interface is a top
priority.

## Implementation

This is a mixed Rust/Qt project:

  - we use Rust for our application engine
  - Qt for the desktop UI

The primary entry point is in multi_main.rs for the app and qml/multi_player.qml.

We're targeting macOS arm64 for our builds right now.

Follow the coding style rules in coding-style.md.

To iterate on this project:

- after updating QML files, run `qmllint`
- after updating Rust files, run `cargo check`