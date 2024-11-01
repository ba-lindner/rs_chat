use std::{
    borrow::Borrow, cell::LazyCell, io::{ErrorKind, Read as _, Write as _}, net::TcpStream
};

#[derive(Debug)]
pub struct Package {
    pub cmd: String,
    pub args: Vec<String>,
}

impl Package {
    pub const ACK: LazyCell<Package> = LazyCell::new(|| Package{
        cmd: "ack".to_string(),
        args: Vec::new(),
    });

    pub fn parse(src: &str) -> Option<Self> {
        let inner = src.strip_prefix(PKG_START)?.strip_suffix(PKG_END)?;
        let (cmd, args) = inner.split_once(CMD_END)?;
        let mut args: Vec<_> = args.split(ARG_END).map(String::from).collect();
        args.pop();
        Some(Self {
            cmd: cmd.to_string(),
            args,
        })
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        [PKG_START, &self.cmd, CMD_END]
            .into_iter()
            .chain(self.args.iter().flat_map(|a| [a, ARG_END]))
            .chain([PKG_END])
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            cmd: "err".to_string(),
            args: vec![msg.into()],
        }
    }

    pub fn msg(channel: impl Into<String>, sender: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            cmd: "msg".to_string(),
            args: vec![
                channel.into(),
                sender.into(),
                msg.into(),
            ],
        }
    }
}

const BUF_SIZE: usize = 256;
const PKG_START: &str = "\u{2}"; // STX
const PKG_END: &str = "\u{3}"; // ETX
const CMD_END: &str = "\u{22}"; // SYN
const ARG_END: &str = "\u{25}"; // EM

pub struct Connection {
    stream: TcpStream,
    buffer: [u8; BUF_SIZE],
    pkg_part: String,
    alive: bool,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Option<Self> {
        stream.set_nonblocking(true).ok()?;
        Some(Self {
            stream,
            buffer: [0; BUF_SIZE],
            pkg_part: String::new(),
            alive: true,
        })
    }

    pub fn alive(&self) -> bool {
        self.alive
    }

    pub fn send_package(&mut self, pkg: impl Borrow<Package>) {
        if !self.alive {
            return;
        }
        for part in pkg.borrow().parts() {
            if self.stream.write_all(part.as_bytes()).is_err() {
                self.alive = false;
                return;
            }
        }
    }

    pub fn get_package(&mut self) -> Option<Package> {
        if !self.alive {
            return None;
        }
        let mut read_bytes = BUF_SIZE;
        while read_bytes == BUF_SIZE {
            read_bytes = match self.stream.read(&mut self.buffer) {
                Ok(bytes) => bytes,
                Err(why) => {
                    if why.kind() != ErrorKind::WouldBlock {
                        self.alive = false;
                        return None;
                    }
                    break;
                }
            };
            if read_bytes == 0 {
                self.alive = false;
                return None;
            }
            self.pkg_part += &String::from_utf8_lossy(&self.buffer[..read_bytes]);
        }
        if let Some(idx) = self.pkg_part.find(PKG_END) {
            let (curr, next) = self.pkg_part.split_at(idx + 1);
            let ret = Package::parse(curr);
            self.pkg_part = String::from(next);
            return ret;
        }
        None
    }
}
