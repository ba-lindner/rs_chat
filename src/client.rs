use std::io::Error;

mod listen;
mod primary;
mod secondary;
mod trivial;

use crate::{connection::Connection, package_enum, requests::Request, SERVER_PORT};

pub use listen::ListenClient;
pub use primary::PrimaryClient;
pub use secondary::SecondaryClient;
pub use trivial::TrivialClient;

fn server_connection(addr: &str, name: &str) -> Result<Connection, ClientErr> {
    let mut conn = Connection::to((addr, SERVER_PORT))?;
    conn.send_package(Request::login(name).package());
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
    StartupFailed,
}

impl From<Error> for ClientErr {
    fn from(value: Error) -> Self {
        Self::IoError(value)
    }
}

package_enum! {
    /// Communication between primary and secondary clients.
    ///
    /// Two groups exist:
    /// * metadata: updates information about name, joined channels and blocked users
    /// * quit: signals the primary client to stop running
    ///
    /// To distinguish between regular packages sent between client and server and
    /// inter-client communication, these commands all have a leading `:`, e.g. `:name`.
    pub enum InterClientComm {
        Name(":name" => name),
        Channels(":channels" => [channels]),
        Blocked(":blocked" => [blocked]),
        Quit(":quit"),
    }
}
