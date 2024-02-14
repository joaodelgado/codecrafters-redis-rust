use std::{io::Cursor, ops::Deref, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use bytes::Buf;

use crate::protocol::{Command, Element, InfoSection, ReplOpt, Set};

pub struct ElementParser<'a> {
    bytes: Cursor<&'a [u8]>,
}

impl<'a> ElementParser<'a> {
    pub fn new(bytes: &'a [u8]) -> ElementParser<'a> {
        ElementParser {
            bytes: Cursor::new(bytes),
        }
    }

    pub fn parse(&mut self) -> Result<Element> {
        match self.read_u8() {
            Some(b'+') => self.read_simple_string(),
            Some(b'$') => self.read_bulk_string(),
            Some(b'*') => self.read_array(),
            Some(other) => bail!("Unsupported element '{}'", other.escape_ascii()),
            None => bail!("Expected at least one byte"),
        }
    }

    fn read_u8(&mut self) -> Option<u8> {
        if !self.bytes.has_remaining() {
            None
        } else {
            Some(self.bytes.get_u8())
        }
    }

    fn consume_byte(&mut self, b: u8) -> Result<()> {
        match self.read_u8() {
            Some(byte_read) if byte_read == b => Ok(()),
            Some(other) => bail!(
                "Expected {}, got {}",
                b.escape_ascii().to_string(),
                other.escape_ascii().to_string()
            ),
            None => bail!(
                "Expected {}, but there are no bytes remaining",
                b.escape_ascii().to_string()
            ),
        }
    }

    fn expect_crlf(&mut self) -> Result<()> {
        self.consume_byte(b'\r')?;
        self.consume_byte(b'\n')
    }

    fn read_simple_string(&mut self) -> Result<Element> {
        let mut buffer = Vec::new();
        loop {
            match self.read_u8() {
                Some(b'\r') => break,
                Some(b) => buffer.push(b),
                None => bail!("buffer terminated before \\r\\n"),
            }
        }

        self.consume_byte(b'\n')?;

        Ok(Element::SimpleString(String::from_utf8(buffer)?))
    }

    fn read_usize_crlf(&mut self) -> Result<usize> {
        let mut value: usize = 0;
        loop {
            match self.read_u8() {
                Some(b) if b.is_ascii_digit() => value = value * 10 + usize::from(b - b'0'),
                Some(b'\r') => break,
                Some(other) => bail!("Expected digit, found {}", other.escape_ascii().to_string()),
                None => bail!("buffer terminated before \\r\\n"),
            }
        }

        self.consume_byte(b'\n')?;

        Ok(value)
    }

    fn read_bulk_string(&mut self) -> Result<Element> {
        let n = self.read_usize_crlf()?;
        if self.bytes.remaining() < n {
            bail!(
                "Attempted to read string of length {}, but buffer only has {} bytes remaining",
                n,
                self.bytes.remaining()
            );
        }

        let s = self.bytes.chunk()[..n].to_vec();
        self.bytes.advance(n);

        self.expect_crlf()?;

        Ok(Element::BulkString(s))
    }

    fn read_array(&mut self) -> Result<Element> {
        let n = self.read_usize_crlf()?;
        let mut elements = Vec::with_capacity(n);

        for _ in 0..n {
            elements.push(self.parse()?)
        }

        Ok(Element::Array(elements))
    }
}

impl TryInto<Command> for Element {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<Command, Self::Error> {
        let args = match self {
            Element::Array(elements) => {
                let mut args = Vec::with_capacity(elements.len());
                for element in elements {
                    match element {
                        Element::BulkString(bytes) => args.push(bytes),
                        other => {
                            bail!("All args of a command must be a bulk string, got {other:?} instead")
                        }
                    }
                }
                args
            }
            _ => bail!("Commands must be an array of bulk strings, got {self:?} instead"),
        };

        if args.is_empty() {
            bail!("Expected at least 1 arg, but got none");
        }

        if args[0].to_ascii_lowercase() == b"ping" {
            return parse_ping(&args[1..]);
        } else if args[0].to_ascii_lowercase() == b"echo" {
            return parse_echo(&args[1..]);
        } else if args[0].to_ascii_lowercase() == b"set" {
            return parse_set(&args[1..]);
        } else if args[0].to_ascii_lowercase() == b"get" {
            return parse_get(&args[1..]);
        } else if args[0].to_ascii_lowercase() == b"info" {
            return parse_info(&args[1..]);
        } else if args[0].to_ascii_lowercase() == b"replconf" {
            return parse_replconf(&args[1..]);
        }

        bail!("Unrecognized command {}", String::from_utf8_lossy(&args[0]));
    }
}

fn parse_ping(args: &[Vec<u8>]) -> Result<Command> {
    match args.first() {
        None => Ok(Command::Ping(None)),
        Some(bytes) => Ok(Command::Ping(Some(String::from_utf8(bytes.clone())?))),
    }
}

fn parse_echo(args: &[Vec<u8>]) -> Result<Command> {
    match args.first() {
        None => bail!("ECHO command requires an argument"),
        Some(bytes) => Ok(Command::Echo(String::from_utf8(bytes.clone())?)),
    }
}

fn parse_set(args: &[Vec<u8>]) -> Result<Command> {
    let mut args = args.iter();

    let key = String::from_utf8(
        args.next()
            .ok_or(anyhow!("missing mandatory key argument"))?
            .clone(),
    )?;
    let value = String::from_utf8(
        args.next()
            .ok_or(anyhow!("missing mandatory value argument"))?
            .clone(),
    )?;
    let mut expiration = None;

    match args.next() {
        Some(arg) if arg.to_ascii_lowercase() == b"px" => {
            let expiration_raw = args.next().ok_or(anyhow!(
                "PX needs to be followed by the expiration date in milliseconds",
            ))?;
            let expiration_millis = String::from_utf8(expiration_raw.clone())
                .context("parsing millis as utf8")?
                .parse()
                .context("parsing millis as number")?;
            expiration = Some(Duration::from_millis(expiration_millis));
        }
        Some(other) => bail!("Unsupported argument {}", String::from_utf8_lossy(other)),
        None => {}
    }

    Ok(Command::Set(Set {
        key,
        value,
        expiration,
    }))
}

fn parse_get(args: &[Vec<u8>]) -> Result<Command> {
    match args.first() {
        None => bail!("GET command requires an argument"),
        Some(bytes) => Ok(Command::Get(String::from_utf8(bytes.clone())?)),
    }
}

fn parse_info(args: &[Vec<u8>]) -> Result<Command> {
    let mut sections = Vec::new();
    for arg in args.iter().map(|arg| arg.to_ascii_lowercase()) {
        match arg.deref() {
            b"replication" => sections.push(InfoSection::Replication),
            other => {
                bail!(
                    "Unsupported info section {}",
                    String::from_utf8_lossy(other)
                );
            }
        }
    }

    Ok(Command::Info(sections))
}

fn parse_replconf(args: &[Vec<u8>]) -> Result<Command> {
    let mut args = args.iter();
    let repl_opt = match args.next().map(|arg| arg.to_ascii_lowercase()).as_deref() {
        Some(b"listening-port") => {
            let port = String::from_utf8(
                args.next()
                    .ok_or(anyhow!(
                        "listening-port replication option requires an argument"
                    ))?
                    .to_vec(),
            )?
            .parse()?;
            ReplOpt::ListeningPort(port)
        }
        Some(b"capa") => {
            let _capability = args
                .next()
                .ok_or(anyhow!("capa replication option requires an argument"))?;
            ReplOpt::Capability
        }
        Some(other) => {
            bail!(
                "Unsupported replication option {}",
                String::from_utf8_lossy(other)
            );
        }
        None => {
            bail!("Expected at least one replication option");
        }
    };

    Ok(Command::ReplConf(repl_opt))
}
