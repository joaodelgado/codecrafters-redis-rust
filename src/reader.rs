use std::{io::Cursor, ops::Deref, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use bytes::Buf;

use crate::protocol::{Command, InfoSection, Set};

pub struct CommandParser<'a> {
    bytes: Cursor<&'a [u8]>,
}

impl<'a> CommandParser<'a> {
    pub fn new(bytes: &'a [u8]) -> CommandParser<'a> {
        CommandParser {
            bytes: Cursor::new(bytes),
        }
    }

    pub fn parse(&mut self) -> Result<Command> {
        self.consume_byte(b'*')?;
        let n = self.read_usize_crlf()?;
        if n == 0 {
            bail!("Expected at least 1 string, but got none");
        }

        let mut strings = Vec::with_capacity(n);
        for _ in 0..n {
            strings.push(self.expect_bulk_string()?);
        }

        if strings[0].to_ascii_lowercase() == b"ping" {
            return parse_ping(&strings[1..]);
        } else if strings[0].to_ascii_lowercase() == b"echo" {
            return parse_echo(&strings[1..]);
        } else if strings[0].to_ascii_lowercase() == b"set" {
            return parse_set(&strings[1..]);
        } else if strings[0].to_ascii_lowercase() == b"get" {
            return parse_get(&strings[1..]);
        } else if strings[0].to_ascii_lowercase() == b"info" {
            return parse_info(&strings[1..]);
        }

        bail!(
            "Unrecognized command {}",
            String::from_utf8_lossy(&strings[0])
        );
    }

    fn expect_bulk_string(&mut self) -> Result<Vec<u8>> {
        self.consume_byte(b'$')?;
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

        Ok(s)
    }

    fn read_u8(&mut self) -> Option<u8> {
        if !self.bytes.has_remaining() {
            None
        } else {
            Some(self.bytes.get_u8())
        }
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
                    String::from_utf8_lossy(&other)
                );
            }
        }
    }

    return Ok(Command::Info(sections));
}
