use crate::protocol::Element;

pub fn serialize(element: Element) -> Vec<u8> {
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
    }
}
