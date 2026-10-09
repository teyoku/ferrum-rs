use crate::{game::run_game_world, net::run_game_server};

mod game;
mod net;

#[tokio::main]
async fn main() {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel(4096);

    tokio::spawn(run_game_server("127.0.0.1:4000", event_tx));

    run_game_world(event_rx).await;
}
