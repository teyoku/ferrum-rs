use std::{
    io::{BufReader, Read, Write},
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

    let _ = event_tx.try_send(GameEvent::Connected {
        id,
        tx: tx_to_player,
    });

    let mut stream_clone = stream.try_clone().expect("Socket cloning error");

    thread::spawn(move || {
        for message in rx_from_game {
            if stream_clone.write_all(message.as_bytes()).is_err() {
                break;
            }
        }

        let _ = stream_clone.shutdown(std::net::Shutdown::Both);
    });

    const MAX_LINE_LEN: usize = 16 * 1024;

    let mut reader = BufReader::new(stream);
    let mut buffer = Vec::new();

    loop {
        if let Some(pos) = buffer.iter().position(|&byte| byte == b'\n') {
            let line = String::from_utf8_lossy(&buffer[..pos]).trim().to_string();
            buffer.drain(..=pos);

            if event_tx
                .try_send(GameEvent::Command { id, text: line })
                .is_err()
            {
                break;
            }

            continue;
        }

        if buffer.len() > MAX_LINE_LEN {
            break;
        }

        let mut chunk = [0u8; 1024];
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let _ = event_tx.try_send(GameEvent::Disconnected { id });
}
