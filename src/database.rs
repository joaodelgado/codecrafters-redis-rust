use std::{collections::HashMap, time::Instant};

use anyhow::Result;
use tokio::sync::RwLock;

use crate::protocol::{Command, Element};

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
struct MasterInfo {
    replication_id: String,
    replication_offset: u128,
}

#[derive(Debug)]
struct ReplicaInfo {
    _master_host: String,
    _master_port: usize,
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
pub struct Database {
    db: RwLock<HashMap<String, Value>>,
    role: Box<dyn RoleInfo + Send + Sync>,
}

impl Database {
    pub async fn execute(&self, command: Command) -> Result<Element> {
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

impl Database {
    pub fn new_master() -> Database {
        Database {
            db: Default::default(),
            role: Box::new(MasterInfo {
                replication_id: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_string(),
                replication_offset: 0,
            }),
        }
    }
}

impl Database {
    pub fn new_replica(master_host: String, master_port: usize) -> Database {
        Database {
            db: Default::default(),
            role: Box::new(ReplicaInfo {
                _master_host: master_host,
                _master_port: master_port,
            }),
        }
    }
}
