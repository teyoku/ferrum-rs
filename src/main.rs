use std::{env, error::Error};

use ferrum_rs::process_client;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        env::set_var("RUST_LOG", "info,warn,error,test");
    }

    dotenvy::dotenv()?;

    let host = env::var("HOST")?;
    let port = env::var("PORT")?;
    let addr = format!("{host}:{port}");

    pretty_env_logger::init();

    let listener = TcpListener::bind(addr).await?;

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(async {
            if let Err(err) = process_client(stream).await {
                println!("An error occurred; error={err:?}");
            }
        });
    }

    Ok(())
}
