use std::{
    io::{BufRead, BufReader, Write},
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

fn handle_client(id: ClientId, mut stream: TcpStream, event_tx: Sender<GameEvent>) {
    let (tx_to_player, rx_from_game) = crossbeam_channel::unbounded::<String>();

    event_tx.send(GameEvent::Connected { id }).unwrap();

    let mut stream_clone = stream.try_clone().expect("Socket cloning error");

    thread::spawn(move || {
        for message in rx_from_game {
            if stream_clone.write_all(message.as_bytes()).is_err() {
                break;
            }
            let _ = stream_clone.flush();
        }
    });

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(text) => {
                let trimmed = text.trim().to_string();
                event_tx.send(GameEvent::Command { id, text }).unwrap();
            }
            Err(_) => break,
        }
    }

    event_tx.send(GameEvent::Disconnected { id }).unwrap();
}
