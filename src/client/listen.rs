use std::io::{stdout, Write};

use super::ClientErr;
use crate::{connection::Connection, requests::Request, response::Response, SERVER_PORT};

pub struct ListenClient {
    conn: Connection,
}

impl ListenClient {
    pub fn connect(addr: &str) -> Result<Self, ClientErr> {
        let mut conn = Connection::to((addr, SERVER_PORT))?;
        conn.send_package(Request::Listen.package());
        if !matches!(
            conn.wait_package().map(|p| p.try_into()),
            Some(Ok(Response::Ack))
        ) || !conn.alive()
        {
            return Err(ClientErr::StartupFailed);
        }
        println!("rs_chat listen client v{}", env!("CARGO_PKG_VERSION"));
        Ok(Self { conn })
    }

    pub fn run(&mut self) {
        while self.conn.alive() {
            if let Some(Ok(Response::Msg(_, name, msg))) =
                self.conn.wait_package().map(|p| p.try_into())
            {
                println!("[{name}] {msg}");
                stdout().flush().unwrap();
            }
        }
    }
}
