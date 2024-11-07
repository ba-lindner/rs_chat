use std::{net::TcpListener, thread, time::Duration};

use crate::{
    connection::Connection,
    package::Package,
    response::Response,
    server::{DIRECT_CHANNEL_NAME, GLOBAL_CHANNEL_NAME},
};

use super::{ClientErr, InterClientComm};

pub struct PrimaryClient {
    server: Connection,
    listener: TcpListener,
    secondary: Option<Connection>,
    name: String,
    channels: Vec<String>,
    blocked: Vec<String>,
}

impl PrimaryClient {
    pub fn connect(addr: &str, name: &str) -> Result<Self, ClientErr> {
        let server = super::server_connection(addr, name)?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let local_port = listener.local_addr()?.port();
        println!("rs_chat primary client v{} running on port {local_port}", env!("CARGO_PKG_VERSION"));
        Ok(Self {
            server,
            listener,
            secondary: None,
            name: name.to_string(),
            channels: vec![String::new()],
            blocked: Vec::new(),
        })
    }

    pub fn run(&mut self) {
        loop {
            if let Some(incoming) = self.server.get_package() {
                if &incoming.cmd == "msg" {
                    Self::print_message(incoming);
                } else if let Some(conn) = &mut self.secondary {
                    conn.send_package(incoming);
                }
            }
            if let Some(conn) = &mut self.secondary {
                if let Some(outgoing) = conn.get_package() {
                    if outgoing.cmd.starts_with(':') {
                        match outgoing.try_into() {
                            Ok(InterClientComm::Channels(channels)) => self.channels = channels,
                            Ok(InterClientComm::Blocked(blocked)) => self.blocked = blocked,
                            Ok(InterClientComm::Quit) => {
                                println!("terminated by user");
                                return;
                            }
                            _ => {}
                        }
                    } else {
                        self.server.send_package(outgoing);
                    }
                }
                if !conn.alive() {
                    self.secondary.take();
                }
            } else if let Ok((stream, _)) = self.listener.accept() {
                if let Ok(mut conn) = Connection::new(stream) {
                    conn.send_package(InterClientComm::Name(self.name.clone()).package());
                    conn.send_package(InterClientComm::Channels(self.channels.clone()).package());
                    conn.send_package(InterClientComm::Blocked(self.blocked.clone()).package());
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
