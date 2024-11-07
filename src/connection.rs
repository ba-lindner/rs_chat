use std::{
    borrow::Borrow,
    io::{Error, ErrorKind, Read as _, Write as _},
    net::{TcpStream, ToSocketAddrs},
    thread,
    time::Duration,
};

use crate::package::Package;

const BUF_SIZE: usize = 256;

pub struct Connection {
    stream: TcpStream,
    buffer: Box<[u8; BUF_SIZE]>,
    pkg_part: String,
    alive: bool,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self, Error> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            buffer: Box::new([0; BUF_SIZE]),
            pkg_part: String::new(),
            alive: true,
        })
    }

    pub fn to(addr: impl ToSocketAddrs) -> Result<Self, Error> {
        Self::new(TcpStream::connect(addr)?)
    }

    pub fn alive(&self) -> bool {
        self.alive
    }

    pub fn send_package(&mut self, pkg: impl Borrow<Package>) {
        if cfg!(debug_assertions) {
            println!("> {:?}", pkg.borrow());
        }
        if !self.alive {
            return;
        }
        let full_pkg: String = pkg.borrow().parts().collect();
        if self.stream.write_all(full_pkg.as_bytes()).is_err() {
            self.alive = false;
        }
    }

    pub fn get_package(&mut self) -> Option<Package> {
        if !self.alive {
            return None;
        }
        let mut read_bytes = BUF_SIZE;
        while read_bytes == BUF_SIZE {
            read_bytes = match self.stream.read(&mut *self.buffer) {
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
        if let Some(idx) = self.pkg_part.find(Package::PKG_END) {
            let (curr, next) = self.pkg_part.split_at(idx + 1);
            let ret = Package::parse(curr);
            self.pkg_part = String::from(next);
            if cfg!(debug_assertions) {
                ret.as_ref().inspect(|pkg| println!("< {pkg:?}"));
            }
            return ret;
        }
        None
    }

    pub fn wait_package(&mut self) -> Option<Package> {
        while self.alive {
            if let Some(pkg) = self.get_package() {
                return Some(pkg);
            }
            thread::sleep(Duration::from_millis(5));
        }
        None
    }
}
