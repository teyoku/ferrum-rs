use std::{env, error::Error};

use ferrum_rs::{ServerActor, load_world, parse_world, process_client};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    pretty_env_logger::init();

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "4000".to_owned());
    let address = format!("{host}:{port}");

    let listener = TcpListener::bind(&address).await?;

    // Embed the example so binaries installed with `cargo install` work even
    // though Cargo does not install package data files beside executables.
    let world = match env::var("WORLD_FILE") {
        Ok(world_file) => load_world(world_file)?,
        Err(_) => parse_world(include_str!("../data/world.json"))?,
    };

    let server = ServerActor::spawn_with_world(world);

    println!("Ferrum MUD is listening on {address}");

    loop {
        let (stream, peer_address) = listener.accept().await?;

        let server = server.clone();

        tokio::spawn(async move {
            println!("Client connected: {peer_address}");

            if let Err(error) = process_client(stream, server).await {
                eprintln!("Client {peer_address} failed: {error}");
            }

            println!("Client disconnected: {peer_address}");
        });
    }
}
