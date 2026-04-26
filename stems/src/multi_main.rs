use anyhow::{Context, Result};
use qmetaobject::*;
use std::process::Command;
use stems::{MultiBridge, WaveformComponent, WaveformView};
use tracing_subscriber;
use walkdir::WalkDir;

mod resources;

fn validate_qml_files() -> Result<()> {
    // Walk through all QML files in the qml directory
    let qmllint_results = WalkDir::new("qml")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|e| e == "qml").unwrap_or(false))
        .map(|e| {
            let file_path = e.path();

            let output = Command::new("qmllint").arg(file_path).output();

            match output {
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    let stdout = String::from_utf8_lossy(&result.stdout);

                    if !result.status.success() || stderr.contains("Error:") {
                        tracing::error!("QML errors in {}: {}", file_path.display(), stderr);
                        return Err(anyhow::anyhow!(
                            "QML errors in {}: {}",
                            file_path.display(),
                            stderr
                        ));
                    } else if !stderr.is_empty() {
                        // Show warnings but don't fail
                        tracing::warn!("QML warnings in {}: {}", file_path.display(), stderr);
                    } else {
                        tracing::debug!("QML file {} passed validation", file_path.display());
                    }

                    if !stdout.is_empty() {
                        tracing::debug!("qmllint output for {}: {}", file_path.display(), stdout);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to run qmllint on {}: {}. Make sure qmllint is installed.",
                        file_path.display(),
                        e
                    );
                }
            }

            Ok(())
        })
        .collect::<Vec<Result<()>>>();

    tracing::info!(
        "QML validation complete: {} files checked",
        qmllint_results.len()
    );

    qmllint_results
        .into_iter()
        .collect::<Result<Vec<()>>>()
        .context("QML validation failed - fix syntax errors before continuing")?;

    Ok(())
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "stems=info,info".to_string()),
        )
        .init();

    tracing::info!("Starting Stems Multi-File Player");

    // Initialize Qt resources
    crate::resources::rsrc();

    // Create multi-bridge
    let mut multi_bridge = MultiBridge::new();

    // Initialize the multi-bridge
    if let Err(e) = multi_bridge.initialize() {
        tracing::error!("Failed to initialize multi-bridge: {}", e);
    }

    // Create QML engine
    let mut engine = QmlEngine::new();

    // Register custom QML components
    qml_register_type::<WaveformView>(cstr::cstr!("StemsUI"), 1, 0, cstr::cstr!("WaveformView"));
    qml_register_type::<WaveformComponent>(
        cstr::cstr!("StemsUI"),
        1,
        0,
        cstr::cstr!("WaveformComponent"),
    );

    // Set context property for QML using a RefCell
    use std::cell::RefCell;
    let bridge_ref = RefCell::new(multi_bridge);
    engine.set_object_property("multiBridge".into(), unsafe {
        QObjectPinned::new(&bridge_ref)
    });

    // Validate all QML files before loading
    validate_qml_files()?;

    // Load QML file
    engine.load_file("qml/multi_player.qml".into());

    // For development, add a timeout to automatically exit and capture state
    if std::env::var("STEMS_DEV_TIMEOUT").is_ok() {
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

    tracing::info!("Multi-File Player exiting");
    Ok(())
}
