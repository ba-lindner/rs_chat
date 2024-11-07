use std::io::Error;

mod primary;
mod secondary;
mod trivial;

use crate::{move_vec, Connection, Package, SERVER_PORT};

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
    StartupFailed,
}

impl From<Error> for ClientErr {
    fn from(value: Error) -> Self {
        Self::IoError(value)
    }
}

/// Communication between primary and secondary clients.
/// 
/// Two groups exist:
/// * metadata: updates information about name, joined channels and blocked users
/// * quit: signals the primary client to stop running
/// 
/// To distinguish between regular packages sent between client and server and
/// inter-client communication, these commands all have a leading `:`, e.g. `:name`.
enum InterClientComm {
    Name(String),
    Channels(Vec<String>),
    Blocked(Vec<String>),
    Quit,
}

impl InterClientComm {
    pub fn into_package(self) -> Package {
        self.into()
    }
}

impl TryFrom<Package> for InterClientComm {
    type Error = ();

    fn try_from(pkg: Package) -> Result<Self, Self::Error> {
        Ok(match pkg.cmd.as_str() {
            ":name" => {
                let [name] = move_vec(pkg.args).ok_or(())?;
                Self::Name(name)
            }
            ":channels" => Self::Channels(pkg.args),
            ":blocked" => Self::Blocked(pkg.args),
            ":quit" => Self::Quit,
            _ => return Err(())
        })
    }
}

impl From<InterClientComm> for Package {
    fn from(value: InterClientComm) -> Self {
        match value {
            InterClientComm::Name(name) => Package { cmd: ":name".to_string(), args: vec![name] },
            InterClientComm::Channels(ch) => Package { cmd: ":channels".to_string(), args: ch },
            InterClientComm::Blocked(bl) => Package { cmd: ":blocked".to_string(), args: bl },
            InterClientComm::Quit => Package { cmd: ":quit".to_string(), args: Vec::new() }
        }
    }
}