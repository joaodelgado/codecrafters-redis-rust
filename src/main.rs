use std::{
    io::{Read, Write},
    net::TcpListener,
};

use anyhow::Result;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => loop {
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf)?;
                stream.write_all(b"+PONG\r\n")?;
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
