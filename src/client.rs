use std::io::Error;

mod primary;
mod secondary;
mod trivial;

use crate::{Connection, Package, SERVER_PORT};

pub use primary::PrimaryClient;
pub use secondary::SecondaryClient;
pub use trivial::TrivialClient;

fn server_connection(addr: &str, name: &str) -> Result<Connection, ClientErr> {
    let mut conn = Connection::to((addr, SERVER_PORT))?;
    conn.send_package(Package {
        cmd: "login".to_string(),
        args: vec![name.to_string()],
    });
    if conn.wait_package().is_some_and(|p| &p.cmd == "ack") {
        Ok(conn)
    } else {
        Err(ClientErr::LoginFailed)
    }
}

#[derive(Debug)]
pub enum ClientErr {
    IoError(Error),
    NonBlockingFailed,
    LoginFailed,
}

impl From<Error> for ClientErr {
    fn from(value: Error) -> Self {
        Self::IoError(value)
    }
}