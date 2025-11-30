use std::net::SocketAddr;

use my_first_package::run_watch_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
    let watch_dir = std::env::current_dir()?;
    run_watch_server(watch_dir, addr).await
}
