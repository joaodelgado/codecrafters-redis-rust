mod database;
mod protocol;
mod reader;
mod writer;

use std::env;

use anyhow::{anyhow, bail, Result};
use database::Database;

#[tokio::main]
async fn main() -> Result<()> {
    let mut port = "6379".to_string();

    let mut is_replica = false;
    let mut master_host = None;
    let mut master_port = None;

    let mut args = env::args().skip(1);
    loop {
        match args.next().as_deref() {
            Some("--port") => port = args.next().ok_or(anyhow!("--port requires an argument"))?,
            Some("--replicaof") => {
                master_host = Some(
                    args.next()
                        .ok_or(anyhow!("--replicaof requires a master host argument"))?,
                );
                master_port = Some(
                    args.next()
                        .ok_or(anyhow!("--replicaof requires a master host argument"))?
                        .parse()?,
                );
                is_replica = true;
            }
            Some(other) => bail!("Unrecognized argument {other}"),
            None => break,
        }
    }

    let address = format!("127.0.0.1:{port}");

    if is_replica {
        Database::new_replica(
            master_host.expect("if we are dealing with a replica, master_host must be set"),
            master_port.expect("if we are dealing with a replica, master_port must be set"),
        )
        .listen(address)
        .await
    } else {
        Database::new_master().listen(address).await
    }
}
