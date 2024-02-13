mod database;
mod protocol;
mod reader;
mod writer;

use std::{env, sync::Arc};

use anyhow::{anyhow, bail, Context, Result};
use database::Database;
use reader::CommandParser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::writer::serialize;

#[tokio::main]
async fn main() -> Result<()> {
    let mut port = "6379".to_string();
    let mut is_replica = false;

    let mut args = env::args().skip(1);
    loop {
        match args.next().as_deref() {
            Some("--port") => port = args.next().ok_or(anyhow!("--port requires an argument"))?,
            Some("--replicaof") => {
                let _ = args
                    .next()
                    .ok_or(anyhow!("--replicaof requires a master host argument"))?;
                let _ = args
                    .next()
                    .ok_or(anyhow!("--replicaof requires a master host argument"))?;
                is_replica = true;
            }
            Some(other) => bail!("Unrecognized argument {other}"),
            None => break,
        }
    }

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .context("creating TCP server")?;

    let database = Arc::new(Database::new(is_replica));

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
        let result = database.execute(command).await?;
        stream.write_all(&serialize(result)).await?;
    }
}
