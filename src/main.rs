use std::{env, error::Error};

use ferrum_rs::{Direction, ServerActor, World, WorldError, process_client};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    pretty_env_logger::init();

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("PORT").unwrap_or_else(|_| "4000".to_owned());
    let address = format!("{host}:{port}");

    let listener = TcpListener::bind(&address).await?;

    let world = build_world()?;
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

fn build_world() -> Result<World, WorldError> {
    let mut world = World::new(
        "Village Square",
        "A stone well stands at the centre of the village.",
    );

    let square = world.start_room();

    let tavern = world.add_room("Old Tavern", "A fire crackles beneath a blackened chimney.")?;

    let forest = world.add_room(
        "Forest Path",
        "A narrow path disappears between ancient trees.",
    )?;

    world.connect_two_way(square, Direction::North, tavern)?;

    world.connect_two_way(square, Direction::East, forest)?;

    Ok(world)
}
