pub mod server;

use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

pub async fn process_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let (reader, mut writer) = stream.into_split();

    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    writer
        .write_all(b"Welcome to Ferrum MUD! Type 'quit' to exit.\r\n")
        .await?;

    loop {
        line.clear();

        let bytes_read = buf_reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            println!("Client disconnected.");
            break;
        }

        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "quit" {
            writer.write_all(b"Goodbye!\r\n").await?;
            break;
        }

        println!("Received from client: {trimmed}");

        let response = format!("You said: {trimmed}\r\n");
        writer.write_all(response.as_bytes()).await?;
    }

    Ok(())
}
