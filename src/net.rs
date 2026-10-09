use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::Sender,
};

use crate::game::event::{ClientId, GameEvent};

pub async fn run_game_server(addr: &str, event_tx: Sender<GameEvent>) {
    let listener = TcpListener::bind(addr).await.expect("Cannot run server");
    let mut id_counter = 0;

    println!("Server running on {addr}");

    loop {
        let (stream, _) = listener.accept().await.expect("Cannot accept connection");
        id_counter += 1;
        let client_id = ClientId::new(id_counter);
        let event_tx_clone = event_tx.clone();

        tokio::spawn(async move { handle_client(client_id, stream, event_tx_clone).await });
    }
}

async fn handle_client(id: ClientId, stream: TcpStream, event_tx: Sender<GameEvent>) {
    let (tx_to_player, mut outbound_rx) = tokio::sync::mpsc::channel(256);

    let _ = event_tx
        .send(GameEvent::Connected {
            id,
            tx: tx_to_player,
        })
        .await;

    const MAX_LINE_LEN: usize = 16 * 1024;
    let (read_half, mut write_half) = stream.into_split();
    let mut lines = BufReader::new(read_half).lines();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.len() > MAX_LINE_LEN {
                            break;
                        }

                        let line = line.trim().to_string();
                        // Keep command delivery bounded and nonblocking.
                        if event_tx.try_send(GameEvent::Command { id, text: line }).is_err() {
                            break;
                        }
                    }
                    Ok(None) | Err(_) => break,
                }
            }
            message = outbound_rx.recv() => {
                match message {
                    Some(message) => {
                        if write_half.write_all(message.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    let _ = write_half.shutdown().await;
    let _ = event_tx.send(GameEvent::Disconnected { id }).await;
}
