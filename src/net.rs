use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use crossbeam_channel::Sender;

use crate::game::event::{ClientId, GameEvent};

pub fn run_game_server(addr: &str, event_tx: Sender<GameEvent>) {
    let listener = TcpListener::bind(addr).expect("Cannot run server");
    let mut id_counter = 0;

    println!("Server running on {addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                id_counter += 1;
                let client_id = ClientId::new(id_counter);
                let event_tx_clone = event_tx.clone();

                thread::spawn(move || {
                    handle_client(client_id, stream, event_tx_clone);
                });
            }
            Err(err) => println!("Connection error: {err}"),
        }
    }
}

fn handle_client(id: ClientId, mut stream: TcpStream, event_tx: Sender<GameEvent>) {}
