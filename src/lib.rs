use std::error::Error;

use tokio::net::TcpStream;

pub async fn process_client(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Processed client");

    Ok(())
}