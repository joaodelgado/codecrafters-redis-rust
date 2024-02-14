use crate::protocol::{Command, Element};

pub fn serialize_command(command: Command) -> Vec<u8> {
    match command {
        Command::Ping(message) => {
            let mut elements = Vec::new();
            elements.push(Element::BulkString(b"PING".to_vec()));
            if let Some(message) = message {
                elements.push(Element::BulkString(message.as_bytes().to_vec()));
            }
            serialize_element(Element::Array(elements))
        }
        Command::Echo(message) => serialize_element(Element::Array(vec![
            Element::BulkString(b"ECHO".to_vec()),
            Element::BulkString(message.as_bytes().to_vec()),
        ])),
        Command::Set(_) => todo!(),
        Command::Get(_) => todo!(),
        Command::Info(_) => todo!(),
    }
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
