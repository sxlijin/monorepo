mod resources;
use qmetaobject::*;
use stems::{player::QtDemoBridge, LatencyReproBridge, LatencyWaveformComponent, Result};
use tracing_subscriber;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "stems=debug,info".to_string()),
        )
        .init();

    tracing::info!("Starting Qt Storybook");

    crate::resources::rsrc();

    // Create minimal bridge for testing Qt integration
    let mut demo_bridge = QtDemoBridge::new();

    // Initialize the demo bridge
    if let Err(e) = demo_bridge.initialize() {
        tracing::error!("Failed to initialize demo bridge: {}", e);
    }

    // Create latency repro bridge for performance testing
    let latency_bridge = LatencyReproBridge::new();

    // Create QML engine
    let mut engine = QmlEngine::new();

    // Register custom QML components
    qml_register_type::<LatencyWaveformComponent>(
        cstr::cstr!("StemsUI"),
        1,
        0,
        cstr::cstr!("LatencyWaveformComponent"),
    );

    // Set context properties for QML using RefCell
    use std::cell::RefCell;
    let demo_bridge_ref = RefCell::new(demo_bridge);
    let latency_bridge_ref = RefCell::new(latency_bridge);

    engine.set_object_property("playerBridge".into(), unsafe {
        QObjectPinned::new(&demo_bridge_ref)
    });
    engine.set_object_property("latencyBridge".into(), unsafe {
        QObjectPinned::new(&latency_bridge_ref)
    });

    // Load QML file
    engine.load_file("qml/storybook/qt_storybook.qml".into());

    // For development, add a timeout to automatically exit and capture state
    if std::env::var("STEMS_DEV_MODE").is_ok() {
        let timeout_secs = std::env::var("STEMS_DEV_TIMEOUT")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u64>()
            .unwrap_or(3);

        tracing::info!("Development mode: will exit after {} seconds", timeout_secs);

        std::thread::spawn(move || {
            // Wait a bit for UI to initialize
            std::thread::sleep(std::time::Duration::from_millis(1000));

            // Wait for remaining time
            std::thread::sleep(std::time::Duration::from_secs(timeout_secs - 1));
            std::process::exit(0);
        });
    }

    // Run the application
    engine.exec();

    tracing::info!("Qt Storybook exiting");
    Ok(())
}
