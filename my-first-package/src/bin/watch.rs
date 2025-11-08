use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
    let app = Router::new()
        .route("/", get(root))
        .route("/ws", get(ws_handler));

    let listener = TcpListener::bind(addr).await?;
    println!("watch CLI serving Axum websocket server on ws://{addr}/ws");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn root() -> impl IntoResponse {
    (StatusCode::OK, "watch server online")
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(stream_watch_events)
}

async fn stream_watch_events(mut socket: WebSocket) {
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    let mut counter: u64 = 0;

    loop {
        ticker.tick().await;
        counter += 1;

        let payload = format!(
            r#"{{"type":"watch-event","sequence":{},"message":"tick {} from CLI"}}"#,
            counter, counter
        );

        if socket.send(Message::Text(payload)).await.is_err() {
            println!("WebSocket client disconnected after {counter} events");
            break;
        }
    }
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    println!("Shutting down watch server");
}
