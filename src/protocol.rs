use std::time::Duration;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Element {
    SimpleString(String),
    BulkString(Vec<u8>),
    NullBulkString,
    Array(Vec<Element>),
}

#[derive(Debug)]
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(Set),
    Get(String),
    Info(Vec<InfoSection>),
}

#[derive(Debug)]
pub struct Set {
    pub key: String,
    pub value: String,
    pub expiration: Option<Duration>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InfoSection {
    Replication,
}
