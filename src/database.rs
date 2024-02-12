use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::RwLock;

use crate::protocol::{Command, Element};

#[derive(Debug, Default)]
pub struct Database {
    db: RwLock<HashMap<String, String>>,
}

impl Database {
    pub async fn execute(&self, command: Command) -> Result<Element> {
        println!("Executing {command:?}");
        match command {
            Command::Ping(message) => Ok(Element::SimpleString(
                message.unwrap_or_else(|| "PONG".to_string()),
            )),
            Command::Echo(message) => Ok(Element::SimpleString(message)),
            Command::Set(key, value) => {
                let mut db = self.db.write().await;
                db.insert(key, value);
                Ok(Element::SimpleString("OK".to_string()))
            }
            Command::Get(key) => {
                let db = self.db.read().await;
                match db.get(&key) {
                    Some(value) => Ok(Element::BulkString(value.as_bytes().to_vec())),
                    None => Ok(Element::NullBulkString),
                }
            }
        }
    }
}
