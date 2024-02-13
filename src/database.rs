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

#[derive(Debug)]
pub struct Database {
    db: RwLock<HashMap<String, Value>>,
    is_replica: bool,
}

impl Database {
    pub fn new(is_replica: bool) -> Database {
        Database {
            db: Default::default(),
            is_replica,
        }
    }

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
                format!("role:{}", if self.is_replica { "slave" } else { "master" })
                    .as_bytes()
                    .to_vec(),
            )),
        }
    }
}
