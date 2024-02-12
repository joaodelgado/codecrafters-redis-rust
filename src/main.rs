mod database;
mod protocol;
mod resp;

use std::sync::Arc;

use anyhow::{Context, Result};
use database::Database;
use resp::CommandParser;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("creating TCP server")?;

    let database = Arc::new(Database::default());

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let database = database.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(database.as_ref(), stream).await {
                        println!("error handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("error accepting connection: {}", e);
            }
        }
    }
}

async fn handle_stream(database: &Database, mut stream: TcpStream) -> Result<()> {
    println!("Client connected");
    loop {
        let mut buf = [0; 1024];
        let n = stream
            .read(&mut buf)
            .await
            .context("read command from client")?;

        if n == 0 {
            println!("Client disconnected");
            return Ok(());
        }

        let command = CommandParser::new(&buf[..n]).parse()?;
        database.execute(command).await?.send(&mut stream).await?;
    }
}
