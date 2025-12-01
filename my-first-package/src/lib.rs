use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub mod tasks;

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use notify_debouncer_full::{
    DebounceEventResult, new_debouncer,
    notify::{RecursiveMode, Watcher},
};
use serde_json::json;
use tokio::{net::TcpListener, signal, sync::broadcast};

#[derive(Clone)]
struct AppState {
    events: broadcast::Sender<String>,
}

pub async fn run_watch_server(
    watch_dir: PathBuf,
    addr: SocketAddr,
    task_selections: Vec<tasks::TaskSelection>,
) -> anyhow::Result<()> {
    let (events_tx, _) = broadcast::channel(512);

    spawn_fs_watcher(watch_dir.clone(), events_tx.clone())?;

    let workspace_root = tasks::find_workspace_root(&watch_dir);
    let task_db = tasks::TaskDb::new(workspace_root.clone());
    for selection in &task_selections {
        task_db.add_root(selection.dir.clone());
        let _ = task_db.reload_root(&selection.dir);
    }

    tasks::TaskScheduler::new(
        task_selections.clone(),
        task_db.clone(),
        watch_dir.clone(),
        events_tx.subscribe(),
    )
    .spawn();

    spawn_task_reloader(
        events_tx.clone(),
        task_db.clone(),
        task_selections.clone(),
        watch_dir.clone(),
    );

    let app_state = AppState { events: events_tx };
    let app = Router::new()
        .route("/", get(root))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let listener = TcpListener::bind(addr).await?;
    println!("watch CLI serving Axum websocket server on ws://{addr}/ws");
    println!("watching {}", watch_dir.display());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn root() -> impl IntoResponse {
    (StatusCode::OK, "watch server online")
}

async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let rx = state.events.subscribe();
    ws.on_upgrade(move |socket| stream_watch_events(socket, rx))
}

async fn stream_watch_events(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    loop {
        match rx.recv().await {
            Ok(payload) => {
                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                let notice = json!({
                    "type": "fs-error",
                    "timestamp": now_millis(),
                    "message": format!("client lagged and skipped {skipped} events"),
                });
                if socket
                    .send(Message::Text(notice.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }

    println!("WebSocket client disconnected");
}

fn spawn_fs_watcher(watch_dir: PathBuf, events: broadcast::Sender<String>) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("watch-fs".into())
        .spawn(move || {
            if let Err(err) = watch_loop(watch_dir, events) {
                eprintln!("file watcher exited: {err:?}");
            }
        })?;

    Ok(())
}

fn watch_loop(watch_dir: PathBuf, events: broadcast::Sender<String>) -> anyhow::Result<()> {
    let (event_tx, event_rx) = mpsc::channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(250), None, move |res| {
        let _ = event_tx.send(res);
    })?;

    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::Recursive)?;

    for result in event_rx {
        match result {
            Ok(events_batch) => {
                for event in events_batch {
                    let paths: Vec<String> = event
                        .paths
                        .iter()
                        .map(|path| relative_path(&watch_dir, path))
                        .collect();

                    let payload = json!({
                        "type": "fs-event",
                        "timestamp": now_millis(),
                        "kind": format!("{:?}", event.kind),
                        "paths": paths,
                    });

                    let _ = events.send(payload.to_string());
                }
            }
            Err(errors) => {
                for error in errors {
                    let payload = json!({
                        "type": "fs-error",
                        "timestamp": now_millis(),
                        "message": error.to_string(),
                    });

                    let _ = events.send(payload.to_string());
                }
            }
        }
    }

    Ok(())
}

pub fn relative_path(root: &Path, candidate: &Path) -> String {
    candidate
        .strip_prefix(root)
        .unwrap_or(candidate)
        .display()
        .to_string()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    println!("Shutting down watch server");
}

fn spawn_task_reloader(
    events: broadcast::Sender<String>,
    task_db: tasks::TaskDb,
    task_selections: Vec<tasks::TaskSelection>,
    watch_root: PathBuf,
) {
    let mut rx = events.subscribe();
    let task_paths: Vec<(PathBuf, String, String)> = task_selections
        .iter()
        .map(|sel| {
            let tasks_path = sel.dir.join(tasks::TASKS_FILE_NAME);
            let rel = relative_path(&watch_root, &tasks_path);
            let trimmed = rel.trim_start_matches("./").to_string();
            (sel.dir.clone(), rel, trimmed)
        })
        .collect();

    tokio::spawn(async move {
        while let Ok(payload) = rx.recv().await {
            if let Some(root) = tasks_file_touched(&payload, &task_paths) {
                let tasks_path = root.join(tasks::TASKS_FILE_NAME);
                if let Err(err) = task_db.reload_root(&root) {
                    eprintln!("{err}");
                } else {
                    println!("reloaded tasks from {}", tasks_path.display());
                }
            }
        }
    });
}

fn tasks_file_touched(payload: &str, task_paths: &[(PathBuf, String, String)]) -> Option<PathBuf> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
        if value.get("type").and_then(|t| t.as_str()) != Some("fs-event") {
            return None;
        }

        if let Some(paths) = value.get("paths").and_then(|p| p.as_array()) {
            for path in paths.iter().filter_map(|p| p.as_str()) {
                let normalized = path.trim_start_matches("./");
                for (root, rel_path, rel_trimmed) in task_paths {
                    if path == rel_path || normalized == rel_trimmed {
                        return Some(root.clone());
                    }
                    if path.ends_with(tasks::TASKS_FILE_NAME)
                        && normalized.ends_with(tasks::TASKS_FILE_NAME)
                        && (path.contains(rel_path) || normalized.contains(rel_trimmed))
                    {
                        return Some(root.clone());
                    }
                }
            }
        }
    }

    None
}
