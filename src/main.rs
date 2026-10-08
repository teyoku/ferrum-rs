use std::thread;

use crate::{game::run_game_world, net::run_game_server};

mod game;
mod net;

fn main() {
    let (event_tx, event_rx) = crossbeam_channel::bounded(4096);

    let net_tx = event_tx.clone();
    thread::spawn(move || {
        run_game_server("127.0.0.1:4000", net_tx);
    });

    run_game_world(event_rx);
}
