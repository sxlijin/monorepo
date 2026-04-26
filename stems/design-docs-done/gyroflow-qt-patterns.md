# Gyroflow Qt/Rust Integration Patterns

Based on analysis of the Gyroflow codebase, this document describes proven patterns for integrating Qt/QML with Rust using qmetaobject-rs.

## Table of Contents
1. [Project Structure](#project-structure)
2. [Dependency Configuration](#dependency-configuration)  
3. [QObject Implementation Patterns](#qobject-implementation-patterns)
4. [Property and Signal Patterns](#property-and-signal-patterns)
5. [QML Bridge Architecture](#qml-bridge-architecture)
6. [Application Lifecycle](#application-lifecycle)
7. [Error Handling](#error-handling)
8. [Performance Considerations](#performance-considerations)
9. [Key Takeaways](#key-takeaways)

---

## Project Structure

Gyroflow organizes Qt/Rust integration with a clear separation:

```
src/
├── main.rs                  # Application entry point and QML engine setup
├── controller.rs            # Main QObject bridge to QML
├── core/                    # Pure Rust business logic
│   ├── stabilization/
│   ├── calibration/
│   └── rendering/
├── qt_gpu/                  # Qt-specific GPU integration (C++)
│   ├── qrhi_undistort.cpp
│   └── *.h files
└── ui/                      # QML files (separate from Rust)
    ├── main.qml
    ├── components/
    └── menu/
```

**Key Pattern**: Separation of concerns between pure Rust logic (core/) and Qt integration layer (controller.rs).

---

## Dependency Configuration

From Gyroflow's Cargo.toml:

```toml
[dependencies]
# Qt integration
qmetaobject = "0.2.10"
qttypes = "0.2.11"  
cpp = "0.5.9"

# Build dependencies
[build-dependencies]
cpp_build = "0.5.9"
```

**Key Pattern**: Gyroflow uses specific versions and includes C++ integration capabilities.

---

## QObject Implementation Patterns

### Basic QObject Structure

Gyroflow's controller.rs shows the standard pattern:

```rust
use qmetaobject::*;

#[derive(QObject, Default)]
pub struct Controller {
    base: qt_base_class!(trait QObject),
    
    // Properties exposed to QML
    pub stabilizer_progress: qt_property!(f64; NOTIFY stabilizer_progress_changed),
    pub current_device: qt_property!(QString; NOTIFY current_device_changed),
    pub has_motion: qt_property!(bool; NOTIFY has_motion_changed),
    
    // Signals  
    stabilizer_progress_changed: qt_signal!(),
    current_device_changed: qt_signal!(),
    has_motion_changed: qt_signal!(),
    request_recompute: qt_signal!(),
    
    // Internal state
    stabilizer: Arc<StabilizationManager>,
    timeline: Arc<Timeline>,
}
```

**Key Patterns**:
1. `#[derive(QObject, Default)]` for automatic QObject implementation
2. `qt_base_class!(trait QObject)` as first field
3. Properties with `NOTIFY` signals for automatic updates
4. Separate internal state using `Arc<>` for thread safety

### Method Implementation

```rust
impl Controller {
    #[qinvokable]
    pub fn load_file(&mut self, path: QString) -> bool {
        let path_str = path.to_string();
        // Implementation
        true
    }
    
    #[qinvokable] 
    pub fn get_smoothing_algorithms(&self) -> QVariantList {
        // Return data to QML
        QVariantList::default()
    }
}
```

**Key Patterns**:
1. `#[qinvokable]` attribute for QML-callable methods
2. Use Qt types (`QString`, `QVariantList`) for QML interface
3. Convert to/from Rust types internally

---

## Property and Signal Patterns

### Property Updates

```rust
impl Controller {
    pub fn set_stabilizer_progress(&mut self, progress: f64) {
        self.stabilizer_progress = progress;
        self.stabilizer_progress_changed(); // Emit signal
    }
    
    pub fn update_current_device(&mut self, device: String) {
        self.current_device = QString::from(device);
        self.current_device_changed();
    }
}
```

**Key Pattern**: Always emit the corresponding signal after updating a property to trigger QML updates.

### Complex Data Passing

```rust
#[qinvokable]
pub fn get_video_info(&self) -> QVariantMap {
    let mut map = QVariantMap::default();
    map.insert("width".into(), QVariant::from(1920i32));
    map.insert("height".into(), QVariant::from(1080i32));
    map.insert("fps".into(), QVariant::from(30.0f64));
    map
}
```

**Key Pattern**: Use `QVariantMap` and `QVariantList` for complex data structures passed to QML.

---

## QML Bridge Architecture

### Application Entry Point

From Gyroflow's main.rs:

```rust
use qmetaobject::*;

fn main() {
    // Initialize QML engine
    let mut engine = QmlEngine::new();
    
    // Create controller
    let mut controller = Controller::new();
    
    // Set global QML context
    engine.set_object_property("controller".into(), 
        QObjectPinned::new(&RefCell::new(controller)));
    
    // Load main QML file
    engine.load_file("ui/main.qml".into());
    
    // Run event loop
    engine.exec();
}
```

**Key Patterns**:
1. Single global controller object exposed to QML
2. Use `QObjectPinned` with `RefCell` for mutable access
3. Load QML from external files for hot reload capability

### QML Integration

In QML files, access Rust controller:

```qml
// ui/main.qml
import QtQuick 2.15

ApplicationWindow {
    property alias controller: controller
    
    Connections {
        target: controller
        function onStabilizer_progress_changed() {
            progressBar.value = controller.stabilizer_progress
        }
    }
    
    Button {
        text: "Load Video"
        onClicked: controller.load_file(fileDialog.selectedFile)
    }
}
```

**Key Pattern**: Use `Connections` for signal handling and direct property access for data binding.

---

## Application Lifecycle

### Initialization Pattern

```rust
impl Controller {
    pub fn new() -> Self {
        let mut controller = Self::default();
        
        // Initialize core components
        controller.stabilizer = Arc::new(StabilizationManager::new());
        controller.timeline = Arc::new(Timeline::new());
        
        // Setup internal connections
        controller.setup_internal_connections();
        
        controller
    }
    
    fn setup_internal_connections(&mut self) {
        // Connect Rust signals to update QML properties
        let stabilizer = Arc::clone(&self.stabilizer);
        // ... setup callbacks
    }
}
```

**Key Pattern**: Separate initialization of Rust components from Qt integration.

### Threading Pattern

```rust
impl Controller {
    #[qinvokable]
    pub fn start_processing(&mut self) {
        let stabilizer = Arc::clone(&self.stabilizer);
        
        std::thread::spawn(move || {
            // Heavy processing in background thread
            stabilizer.process();
        });
    }
}
```

**Key Pattern**: Use background threads for heavy work, communicate back via signals.

---

## Error Handling

### QML Error Reporting

```rust
impl Controller {
    error_occurred: qt_signal!(message: QString),
    
    #[qinvokable]
    pub fn risky_operation(&mut self) -> bool {
        match self.do_something() {
            Ok(_) => true,
            Err(e) => {
                self.error_occurred(QString::from(format!("Error: {}", e)));
                false
            }
        }
    }
}
```

**Key Pattern**: Use signals to communicate errors back to QML for user display.

---

## Performance Considerations

### Efficient Data Updates

```rust
impl Controller {
    // Batch property updates
    pub fn update_video_info(&mut self, width: i32, height: i32, fps: f64) {
        self.video_width = width;
        self.video_height = height; 
        self.video_fps = fps;
        
        // Single signal for multiple related changes
        self.video_info_changed();
    }
}
```

**Key Pattern**: Batch related property updates and emit single signals to reduce QML recomputation.

### Memory Management

```rust
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(QObject, Default)]
pub struct Controller {
    // Use Arc for shared ownership
    stabilizer: Arc<RwLock<StabilizationManager>>,
    
    // Use Qt types for QML interface only
    current_file: qt_property!(QString; NOTIFY current_file_changed),
}
```

**Key Pattern**: Use `Arc` and `RwLock` for internal state, Qt types only at the interface boundary.

---

## Key Takeaways

### 1. Architecture Principles
- **Separation of Concerns**: Keep pure Rust logic separate from Qt integration
- **Single Bridge**: Use one main controller QObject as the primary QML bridge
- **Signal-Driven**: Use Qt signals for all communication from Rust to QML

### 2. Implementation Patterns
- **Property-Signal Pairs**: Every property should have a corresponding change signal
- **Qt Types at Boundary**: Use Qt types (`QString`, `QVariantMap`) only at QML interface
- **Thread Safety**: Use `Arc` and appropriate synchronization for shared state

### 3. Development Workflow
- **Hot Reload**: Structure allows QML hot reload for rapid UI iteration
- **Testable Core**: Pure Rust logic can be unit tested independently
- **Incremental Development**: Can build UI and logic incrementally

### 4. Performance Best Practices
- **Batch Updates**: Group related property changes into single signals
- **Background Processing**: Keep heavy work off the main thread
- **Efficient Data Structures**: Use appropriate collection types for QML data

### 5. Common Patterns to Avoid
- **Direct State Mutation**: Always go through controller methods
- **Complex QML Logic**: Keep business logic in Rust, not QML
- **Blocking Operations**: Never block the main thread from QML calls

This analysis shows that Gyroflow's approach provides a robust, maintainable pattern for Qt/Rust integration that balances performance with development velocity.