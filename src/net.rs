use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc::Sender,
};

use crate::game::event::{ClientId, GameEvent};

pub async fn run_game_server(addr: &str, event_tx: Sender<GameEvent>) {
    let listener = TcpListener::bind(addr).await.expect("Cannot run server");
    let mut id_counter = 0;

    println!("Server running on {addr}");

    for (stream, _) in listener.accept().await {
        id_counter += 1;
        let client_id = ClientId::new(id_counter);
        let event_tx_clone = event_tx.clone();

        tokio::spawn(async move { handle_client(client_id, stream, event_tx_clone).await });
    }
}

async fn handle_client(id: ClientId, mut stream: TcpStream, event_tx: Sender<GameEvent>) {
    let (tx_to_player, mut rx_from_game) = tokio::sync::mpsc::channel(256);

    let _ = event_tx
        .send(GameEvent::Connected {
            id,
            tx: tx_to_player,
        })
        .await;

    while let Some(message) = rx_from_game.recv().await {
        if stream.write_all(message.as_bytes()).await.is_err() {
            break;
        }
    }

    let _ = stream.shutdown().await;

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
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let _ = event_tx.send(GameEvent::Disconnected { id }).await;
}
