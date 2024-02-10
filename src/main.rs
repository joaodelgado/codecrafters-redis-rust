use anyhow::{Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
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
    loop {
        let mut buf = [0; 1024];
        let _ = stream
            .read(&mut buf)
            .await
            .context("read command from client")?;
        stream
            .write_all(b"+PONG\r\n")
            .await
            .context("write result to client")?;
    }
}
