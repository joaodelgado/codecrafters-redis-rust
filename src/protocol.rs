use anyhow::Result;
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Element {
    SimpleString(String),
    BulkString(Vec<u8>),
    NullBulkString,
}

impl Element {
    pub async fn send(self, stream: &mut TcpStream) -> Result<()> {
        match self {
            Element::SimpleString(message) => stream
                .write_all(format!("+{}\r\n", message).as_bytes())
                .await
                .map_err(Into::into),
            Element::BulkString(data) => {
                stream
                    .write_all(format!("${}\r\n", data.len()).as_bytes())
                    .await?;
                stream.write_all(&data).await?;
                stream.write_all(b"\r\n").await?;
                Ok(())
            }
            Element::NullBulkString => stream.write_all(b"$-1\r\n").await.map_err(Into::into),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(String, String),
    Get(String),
}
