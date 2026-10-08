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

fn handle_client(id: ClientId, stream: TcpStream, event_tx: Sender<GameEvent>) {
    let (tx_to_player, rx_from_game) = crossbeam_channel::bounded(256);

    event_tx
        .try_send(GameEvent::Connected {
            id,
            tx: tx_to_player,
        })
        .unwrap();

    let mut stream_clone = stream.try_clone().expect("Socket cloning error");

    thread::spawn(move || {
        for message in rx_from_game {
            if stream_clone.write_all(message.as_bytes()).is_err() {
                break;
            }
        }

        let _ = stream_clone.shutdown(std::net::Shutdown::Both);
    });

    let reader = BufReader::new(stream);
    for line in reader.lines() {
        match line {
            Ok(text) => {
                let trimmed = text.trim().to_string();

                if event_tx
                    .send(GameEvent::Command { id, text: trimmed })
                    .is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = event_tx.send(GameEvent::Disconnected { id });
}
