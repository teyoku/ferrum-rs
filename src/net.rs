use std::net::TcpListener;

use crossbeam_channel::Sender;

pub fn run_game_server(addr: &str, event_tx: Sender<_>) {
    let listener = TcpListener::bind(addr).expect("Cannot run server");
    let mut id_counter = 0;

    println!("Server running on {addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                id_counter += 1;
            }
            Err(err) => println!("Connection error: {err}")
        }
    }
}