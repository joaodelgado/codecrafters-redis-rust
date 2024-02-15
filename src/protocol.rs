use std::time::Duration;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Element {
    SimpleString(String),
    BulkString(Vec<u8>),
    NullBulkString,
    Array(Vec<Element>),
    RdbFile(Vec<u8>),
    MultiInternal(Vec<Element>),
}

#[derive(Debug)]
pub enum Command {
    Ping(Option<String>),
    Echo(String),
    Set(Set),
    Get(String),
    Info(Vec<InfoSection>),
    ReplConf(ReplOpt),
    Psync(Psync),
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

#[derive(Debug, PartialEq, Eq)]
pub enum ReplOpt {
    ListeningPort(usize),
    Capability,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Psync {
    pub replication_id: Option<String>,
    pub replication_offset: Option<u128>,
}
