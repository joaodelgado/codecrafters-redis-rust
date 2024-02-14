use crate::protocol::{Command, Element, ReplOpt};

pub fn serialize_command(command: Command) -> Vec<u8> {
    let args = match command {
        Command::Ping(message) => {
            let mut elements = Vec::new();
            elements.push(Element::BulkString(b"PING".to_vec()));
            if let Some(message) = message {
                elements.push(Element::BulkString(message.as_bytes().to_vec()));
            }
            elements
        }
        Command::Echo(message) => vec![
            Element::BulkString(b"ECHO".to_vec()),
            Element::BulkString(message.as_bytes().to_vec()),
        ],
        Command::Set(_) => todo!(),
        Command::Get(_) => todo!(),
        Command::Info(_) => todo!(),
        Command::ReplConf(repl_opt) => {
            let mut args = vec![Element::BulkString(b"REPLCONF".to_vec())];
            match repl_opt {
                ReplOpt::ListeningPort(port) => {
                    args.push(Element::BulkString(b"listening-port".to_vec()));
                    args.push(Element::BulkString(format!("{port}").into()));
                }
                ReplOpt::Capability => {
                    args.push(Element::BulkString(b"capa".to_vec()));
                    args.push(Element::BulkString(b"psync2".to_vec()));
                }
            }
            args
        }
    };
    serialize_element(Element::Array(args))
}

pub fn serialize_element(element: Element) -> Vec<u8> {
    match element {
        Element::SimpleString(message) => format!("+{}\r\n", message).as_bytes().to_vec(),
        Element::BulkString(data) => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(format!("${}\r\n", data.len()).as_bytes());
            bytes.extend_from_slice(&data);
            bytes.extend_from_slice(b"\r\n");
            bytes
        }
        Element::NullBulkString => b"$-1\r\n".to_vec(),
        Element::Array(elements) => {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(format!("*{}\r\n", elements.len()).as_bytes());
            for element in elements.into_iter() {
                bytes.extend_from_slice(&serialize_element(element))
            }
            bytes
        }
    }
}
