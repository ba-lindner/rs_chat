use std::{net::TcpListener, thread, time::Duration};

use crate::{response::Response, server::{GLOBAL_CHANNEL_NAME, DIRECT_CHANNEL_NAME}, Connection, Package};

use super::ClientErr;

pub struct PrimaryClient {
    server: Connection,
    listener: TcpListener,
    secondary: Option<Connection>,
}

impl PrimaryClient {
    pub fn connect(addr: &str, name: &str) -> Result<Self, ClientErr> {
        let server = super::server_connection(addr, name)?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let local_port = listener.local_addr()?.port();
        println!("rs_chat primary client running on port {local_port}");
        Ok(Self {
            server,
            listener,
            secondary: None,
        })
    }

    pub fn run(&mut self) {
        loop {
            if let Some(conn) = &mut self.secondary {
                if let Some(incoming) = self.server.get_package() {
                    if &incoming.cmd == "msg" {
                        Self::print_message(incoming);
                    } else {
                        conn.send_package(incoming);
                    }
                }
                if let Some(outgoing) = conn.get_package() {
                    self.server.send_package(outgoing);
                }
                if !conn.alive() {
                    self.secondary.take();
                }
            } else if let Ok((stream, _)) = self.listener.accept() {
                if let Ok(conn) = Connection::new(stream) {
                    self.secondary = Some(conn);
                }
            }
            if !self.server.alive() {
                eprintln!("disconnected from server");
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn print_message(msg: Package) {
        let Ok(Response::Msg(channel, mut sender, msg)) = msg.try_into() else {
            eprintln!("server sent invalid response");
            return;
        };
        let ch = match channel.as_str() {
            GLOBAL_CHANNEL_NAME => GLOBAL_CHANNEL_NAME,
            DIRECT_CHANNEL_NAME => " -> you",
            _ => {
                sender.push('@');
                &channel
            }
        };
        println!("[{sender}{ch}] {msg}");
    }
}
