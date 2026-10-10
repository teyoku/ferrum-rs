use std::{env, error::Error};

use ferrum_rs::{ServerActor, process_client};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // A local .env file is optional; environment variables may also be
    // supplied by the shell or a process manager.
    dotenvy::dotenv().ok();

    pretty_env_logger::init();

    // Keep development defaults convenient while allowing deployments to
    // select a different interface and port.
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "4000".to_owned());
    let address = format!("{host}:{port}");

    let listener = TcpListener::bind(&address).await?;

    // Spawning the actor starts its mailbox loop and returns a lightweight,
    // cloneable handle used by client tasks to send commands to it.
    let server = ServerActor::spawn();

    println!("Ferrum MUD is listening on {address}");

    loop {
        let (stream, peer_address) = listener.accept().await?;

        // Cloning ServerHandle clones only the channel sender. The actor and
        // its session map continue to have a single owner.
        let server = server.clone();

        // Each connection gets an independent asynchronous task so one slow
        // client cannot stop the listener from accepting other clients.
        tokio::spawn(async move {
            println!("Client connected: {peer_address}");

            if let Err(error) = process_client(stream, server).await {
                eprintln!("Client {peer_address} failed: {error}");
            }

            println!("Client disconnected: {peer_address}");
        });
    }
}
