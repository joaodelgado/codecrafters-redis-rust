use std::{collections::HashMap, sync::Arc, time::Instant};

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    protocol::{Command, Element},
    reader::ElementParser,
    writer::{serialize_command, serialize_element},
};

#[derive(Debug)]
struct Value {
    value: String,
    expiration: Option<Instant>,
}

impl Value {
    fn is_expired(&self) -> bool {
        match self.expiration {
            None => false,
            Some(expiration) => Instant::now() > expiration,
        }
    }
}

pub trait RoleInfo: std::fmt::Debug {
    fn as_info_section(&self) -> String;
}

#[derive(Debug)]
pub struct MasterInfo {
    replication_id: String,
    replication_offset: u128,
}

#[derive(Debug)]
pub struct ReplicaInfo {
    master: TcpStream,
}

impl RoleInfo for MasterInfo {
    fn as_info_section(&self) -> String {
        format!(
            "role:master
master_replid:{}
master_repl_offset:{}
",
            self.replication_id, self.replication_offset
        )
    }
}

impl RoleInfo for ReplicaInfo {
    fn as_info_section(&self) -> String {
        "role:slave".to_string()
    }
}

#[derive(Debug)]
pub struct Database<W: Send> {
    port: usize,
    db: RwLock<HashMap<String, Value>>,
    role: W,
}

impl<W: RoleInfo + Send + Sync + 'static> Database<W> {
    pub async fn listen(self) -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))
            .await
            .context("creating TCP server")?;
        println!("Listening on port {}", self.port);

        let arc_self = Arc::new(self);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let s = arc_self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = s.handle_stream(stream).await {
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

    async fn handle_stream(&self, mut stream: TcpStream) -> Result<()> {
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

            let command = ElementParser::new(&buf[..n]).parse()?.try_into()?;
            let result = self.execute(command).await?;
            stream.write_all(&serialize_element(result)).await?;
        }
    }

    async fn execute(&self, command: Command) -> Result<Element> {
        println!("Executing {command:?}");
        match command {
            Command::Ping(message) => Ok(Element::SimpleString(
                message.unwrap_or_else(|| "PONG".to_string()),
            )),
            Command::Echo(message) => Ok(Element::SimpleString(message)),
            Command::Set(set) => {
                let mut db = self.db.write().await;
                db.insert(
                    set.key,
                    Value {
                        value: set.value,
                        expiration: set.expiration.map(|expiration| Instant::now() + expiration),
                    },
                );
                Ok(Element::SimpleString("OK".to_string()))
            }
            Command::Get(key) => {
                let db = self.db.read().await;
                match db.get(&key) {
                    Some(value) if !value.is_expired() => {
                        Ok(Element::BulkString(value.value.as_bytes().to_vec()))
                    }
                    _ => Ok(Element::NullBulkString),
                }
            }
            Command::Info(_section) => Ok(Element::BulkString(
                self.role.as_info_section().as_bytes().to_vec(),
            )),
        }
    }
}

impl Database<MasterInfo> {
    pub fn new_master(port: usize) -> Self {
        Database {
            port,
            db: Default::default(),
            role: MasterInfo {
                replication_id: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string(),
                replication_offset: 0,
            },
        }
    }
}

impl Database<ReplicaInfo> {
    pub async fn new_replica(port: usize, master_host: String, master_port: usize) -> Result<Self> {
        let mut database = Database {
            port,
            db: Default::default(),
            role: ReplicaInfo {
                master: TcpStream::connect(format!("{master_host}:{master_port}")).await?,
            },
        };

        database.handshake().await?;

        Ok(database)
    }

    async fn handshake(&mut self) -> Result<()> {
        println!("Handshaking with master");

        self.send_command_to_master(Command::Ping(None)).await?;

        Ok(())
    }

    async fn send_command_to_master(&mut self, command: Command) -> Result<Element> {
        println!("Sending {command:?}");
        self.role
            .master
            .write_all(&serialize_command(command))
            .await?;
        let mut response = [0; 1024];
        let n = self.role.master.read(&mut response).await?;
        let result = ElementParser::new(&response[..n]).parse();

        println!("Got {result:?}");
        result
    }
}
