mod processor;
mod resp;

use anyhow::{Context, Result};
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

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(stream).await {
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

async fn handle_stream(mut stream: TcpStream) -> Result<()> {
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

        CommandParser::new(&buf[..n])
            .parse()?
            .execute()?
            .send(&mut stream)
            .await?;
    }
}
