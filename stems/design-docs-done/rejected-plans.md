# Rejected Implementation Plans

This document captures the alternative approaches, frameworks, and architectural decisions that were considered but ultimately rejected for the stems player project. Understanding these trade-offs provides valuable context for future development decisions and helps avoid revisiting settled questions.

## GUI Framework Alternatives

### Rejected: egui (Immediate Mode GUI)

**Why Considered**:
- Pure Rust implementation
- Excellent for real-time audio visualizations
- Simple mental model, easy to debug
- Great performance with minimal allocations

**Why Rejected**:
- **Non-native appearance** - Custom styling doesn't match macOS conventions
- **State management challenges** - Immediate mode makes complex UI state harder to manage
- **Limited built-in widgets** - Would need to implement audio-specific controls from scratch
- **No hot reload** - Slower iteration compared to QML's live reload capability

**Final Assessment**: While egui excels at real-time visualizations, the lack of native platform integration and development velocity concerns outweighed its performance benefits.

### Rejected: iced (Elm-Inspired GUI)

**Why Considered**:
- Clean Elm architecture pattern
- Good hybrid support (retained state + canvas for custom drawing)
- Pure Rust with GPU acceleration
- Excellent canvas API for waveform rendering

**Why Rejected**:
- **No audio-specific components** - Would need to build everything from scratch
- **Limited real-time optimization** - Not designed with audio application constraints in mind
- **Younger ecosystem** - Fewer examples and community resources compared to Qt
- **Uncertain performance** - No proven track record for complex audio applications

**Final Assessment**: Strong technical foundation but lacks the battle-tested reliability and audio-specific optimizations needed for professional media applications.

### Rejected: Tauri (Web Technologies + Rust Backend)

**Why Considered**:
- Familiar web development model (HTML/CSS/JS)
- Rich visualization ecosystem (Canvas, WebGL, charting libraries)
- Good separation between UI and audio processing
- Rapid development using existing web UI libraries

**Why Rejected**:
- **Audio latency concerns** - Web layer adds unacceptable latency for real-time audio UI updates
- **Limited real-time capability** - JavaScript timing not suitable for sample-accurate operations
- **Complex architecture** - IPC overhead between UI and audio processes
- **Platform integration limitations** - Web sandbox restricts access to native audio APIs

**Final Assessment**: While attractive for rapid prototyping, the fundamental architectural limitations of web technologies make it unsuitable for professional audio applications requiring real-time performance.

### Rejected: slint (Declarative UI Toolkit)

**Why Considered**:
- Modern declarative syntax similar to QML
- Good performance with compiled UI definitions
- Design tool support with visual editor
- Pure Rust implementation

**Why Rejected**:
- **Very young framework** - Limited battle-testing and production applications
- **Unknown audio suitability** - No proven track record with audio or media applications
- **Limited community** - Small ecosystem compared to established frameworks
- **Uncertain long-term support** - Risk of framework abandonment or major breaking changes

**Final Assessment**: Too experimental for a project requiring proven reliability and performance characteristics.

### Rejected: gtk4-rs (GNOME/GTK)

**Why Considered**:
- Mature and stable framework with decades of development
- Good multimedia support through GStreamer integration
- Pure Rust bindings available
- Strong accessibility support

**Why Rejected**:
- **Non-native on macOS** - Looks and feels foreign to macOS users
- **Complex styling system** - CSS-based theming doesn't match native macOS appearance
- **Performance considerations** - Not optimized for real-time audio visualizations
- **Platform mismatch** - Better suited for Linux environments than macOS

**Final Assessment**: While technically capable, poor macOS integration makes it unsuitable for a macOS-focused application.

### Rejected: cacao (Pure macOS Cocoa Bindings)

**Why Considered**:
- True macOS native experience with direct Cocoa bindings
- Pure Rust implementation without C++ interop
- Perfect platform integration with Core Audio and Metal
- Optimal performance for macOS-specific features

**Why Rejected**:
- **Platform lock-in** - macOS-only framework limits future cross-platform potential
- **Build everything from scratch** - No audio-specific components or multimedia framework
- **Smaller ecosystem** - Limited examples and community compared to Qt
- **Development velocity** - Would require implementing many UI components manually

**Final Assessment**: While offering the best native macOS experience, the development overhead and platform limitations outweighed the benefits when a proven cross-platform solution exists.

## Audio Library Alternatives

### Rejected: JUCE Framework (C++)

**Why Considered**:
- Most battle-tested framework for audio applications
- Used by major DAWs and thousands of audio plugins
- Built-in audio components (waveform display, device management)
- Excellent real-time performance and plugin ecosystem

**Why Rejected**:
- **Licensing complexity** - GPL vs commercial licensing issues for closed-source distribution
- **C++ integration overhead** - Significant complexity in Rust/C++ interop via cxx crate
- **Heavy framework** - Large binary size and many dependencies for a simple player
- **Development velocity** - Slower iteration compared to pure Rust solutions

https://www.reddit.com/r/cpp/comments/1ehd6vq/why_do_some_devs_go_with_qt_instead_of_juce_for/

**Final Assessment**: While technically excellent, the licensing concerns and development complexity made it unsuitable for this project's goals.

### Rejected: Pure Web Audio (Tauri + Web Audio API)

**Why Considered**:
- Familiar web development model
- Rich visualization capabilities with Canvas/WebGL
- Built-in audio synchronization via AudioContext
- Extensive ecosystem of audio processing libraries

**Why Rejected**:
- **Latency limitations** - Web Audio API adds unacceptable latency for professional use
- **Limited file access** - Browser sandbox restrictions on direct file system access
- **Performance constraints** - JavaScript overhead impacts real-time audio processing
- **Platform integration** - Cannot access native audio APIs directly

**Final Assessment**: Fundamental limitations of web platform make it unsuitable for professional audio applications.

### Rejected: rodio (Pure Rust Audio)

**Why Considered**:
- Pure Rust implementation with no C++ dependencies
- Simple API perfect for basic audio playback
- Good cross-platform support
- Lightweight and easy to integrate

**Why Rejected**:
- **Limited real-time control** - Not designed for sample-accurate synchronization
- **Basic feature set** - Lacks advanced audio processing capabilities needed for multi-stem sync
- **Performance limitations** - Not optimized for professional audio latency requirements
- **Limited platform optimization** - Doesn't leverage platform-specific audio optimizations

**Final Assessment**: While excellent for simple use cases, lacks the precision and performance needed for professional multi-stem synchronization.

## Architecture Pattern Alternatives

### Rejected: Pure Immediate Mode (Dear ImGui Style)

**Why Considered**:
- Perfect for real-time audio visualizations
- Simple mental model with no persistent state
- Used successfully in many audio debugging tools
- Excellent performance for constantly updating displays

**Why Rejected**:
- **Non-native appearance** - Game/tool aesthetic doesn't match desktop applications
- **Limited layout capabilities** - Primarily designed for tooling interfaces, not media players
- **State management complexity** - Application state becomes difficult to manage at scale
- **User experience** - Interface paradigm unfamiliar to typical media application users

**Final Assessment**: While technically sound for visualization components, inappropriate for the overall application architecture.

### Rejected: Actor Model Architecture

**Why Considered**:
- Excellent isolation between audio and UI concerns
- Natural fit for Rust's ownership model
- Clear message-passing interfaces
- Good scalability for complex audio processing

**Why Rejected**:
- **Increased complexity** - Overhead of message passing for simple operations
- **Debugging difficulty** - Distributed state makes issues harder to trace
- **Performance overhead** - Message serialization adds latency to critical paths
- **Development velocity** - More complex to implement and test than simpler approaches

**Final Assessment**: Over-engineered for the relatively straightforward requirements of a stem player application.

### Rejected: Plugin Architecture

**Why Considered**:
- Extensibility for future audio effects and processing
- Clean separation of concerns
- Industry standard for audio applications
- Professional workflow integration

**Why Rejected**:
- **Complexity overhead** - Significant architecture complexity for uncertain future benefits
- **Scope creep risk** - Feature complexity beyond project requirements
- **Development time** - Substantial additional development effort
- **YAGNI principle** - No current requirement for plugin functionality

**Final Assessment**: While appealing for future extensibility, adds unnecessary complexity to the initial implementation.

## Development Approach Alternatives

### Rejected: Test-Driven Development (TDD)

**Why Considered**:
- High confidence in correctness
- Clear requirements definition
- Regression prevention
- Industry best practice for reliable software

**Why Rejected**:
- **Exploration phase** - Initial development requires experimentation with audio APIs
- **Integration complexity** - Audio/UI integration difficult to test in isolation
- **Performance focus** - Real-time constraints better validated through profiling than unit tests
- **Development velocity** - TDD overhead slows initial prototyping and learning

**Final Assessment**: While valuable for production software, TDD adds overhead during the exploratory development phase. Testing will be added after core architecture is established.

### Rejected: Microservice Architecture

**Why Considered**:
- Clear separation between audio engine and UI
- Independent scaling and deployment
- Language flexibility for different components
- Modern architectural pattern

**Why Rejected**:
- **Latency overhead** - Network/IPC latency unacceptable for real-time audio
- **Complexity overhead** - Distributed systems complexity for single-user desktop application
- **Resource usage** - Multiple processes increase memory and CPU overhead
- **Development complexity** - Service discovery, error handling, deployment complexity

**Final Assessment**: Microservices add complexity without benefits for desktop audio applications requiring real-time performance.

### Rejected: Functional Programming Approach

**Why Considered**:
- Immutable state reduces synchronization bugs
- Pure functions easier to test and reason about
- Natural fit for audio signal processing
- Rust's functional programming capabilities

**Why Rejected**:
- **Performance overhead** - Immutable data structures add memory allocation pressure
- **Real-time constraints** - Garbage collection or allocation during audio callbacks problematic
- **Platform integration** - Audio APIs designed around mutable state and callbacks
- **Learning curve** - FP paradigm adds complexity for contributors familiar with imperative audio code

**Final Assessment**: While intellectually appealing, real-time audio applications require careful memory management that conflicts with pure functional approaches.

## Platform Strategy Alternatives

### Rejected: Cross-Platform First

**Why Considered**:
- Broader market reach
- Code reuse across platforms
- Future-proofing for Windows/Linux support
- Efficient development resources

**Why Rejected**:
- **Complexity overhead** - Abstraction layers reduce platform-specific optimizations
- **Lowest common denominator** - Cross-platform APIs often miss best platform-specific features
- **Development focus** - Dilutes effort across multiple platforms during initial development
- **User experience** - Platform-specific optimizations provide better user experience

**Final Assessment**: macOS-first approach allows leveraging platform strengths (Core Audio, Metal) for optimal user experience, with cross-platform potential preserved for future consideration.

### Rejected: Web-First Strategy

**Why Considered**:
- Universal accessibility through browsers
- Single codebase for all platforms
- Familiar development model
- Rich ecosystem of web technologies

**Why Rejected**:
- **Performance limitations** - Web platform cannot achieve professional audio latency requirements
- **File system restrictions** - Browser sandbox limits direct file access needed for local audio files
- **Platform integration** - Cannot access native audio APIs for optimal performance
- **User experience** - Web applications feel less polished than native desktop applications

**Final Assessment**: Web platform limitations make it fundamentally unsuitable for professional audio applications requiring direct hardware access and real-time performance.

## Conclusion

The rejected alternatives represent a comprehensive exploration of available technologies and approaches. The final choice of qmetaobject-rs + QML provides the optimal balance of:

- **Proven reliability** (demonstrated by Gyroflow's success)
- **Development velocity** (QML hot reload, familiar patterns)
- **Performance characteristics** (real-time audio, hardware acceleration)
- **Platform integration** (native macOS experience)
- **Future flexibility** (cross-platform potential preserved)

This decision provides a solid foundation for building a professional-grade stem player while maintaining reasonable development complexity and timeline constraints.