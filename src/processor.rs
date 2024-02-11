use anyhow::Result;
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[derive(Debug)]
pub enum Element {
    SimpleString(String),
}

impl Element {
    pub async fn send(self, stream: &mut TcpStream) -> Result<()> {
        match self {
            Element::SimpleString(message) => stream
                .write_all(format!("+{}\r\n", message).as_bytes())
                .await
                .map_err(Into::into),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping(Option<String>),
    Echo(String),
}

impl Command {
    pub fn execute(self) -> Result<Element> {
        println!("Executing {self:?}");
        match self {
            Command::Ping(message) => Ok(Element::SimpleString(
                message.unwrap_or_else(|| "PONG".to_string()),
            )),
            Command::Echo(message) => Ok(Element::SimpleString(message)),
        }
    }
}
