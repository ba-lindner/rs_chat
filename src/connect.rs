use std::{
    borrow::Borrow,
    fmt::Display,
    io::{Error, ErrorKind, Read as _, Write as _},
    net::{TcpStream, ToSocketAddrs},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Package {
    pub cmd: String,
    pub args: Vec<String>,
}

impl Package {
    const PKG_START: &'static str = "\x02"; // STX
    const CMD_END: &'static str = "\x16"; // SYN
    const ARG_END: &'static str = "\x19"; // EM
    const PKG_END: &'static str = "\x03"; // ETX

    pub fn parse(src: &str) -> Option<Self> {
        let inner = src
            .strip_prefix(Self::PKG_START)?
            .strip_suffix(Self::PKG_END)?;
        let (cmd, args) = inner.split_once(Self::CMD_END)?;
        let mut args: Vec<_> = args.split(Self::ARG_END).map(String::from).collect();
        args.pop();
        Some(Self {
            cmd: cmd.to_string(),
            args,
        })
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        [Self::PKG_START, &self.cmd, Self::CMD_END]
            .into_iter()
            .chain(self.args.iter().flat_map(|a| [a, Self::ARG_END]))
            .chain([Self::PKG_END])
    }
}

#[derive(Debug)]
pub enum PackageParseError {
    UnknownCmd(String),
    MissingArgs(&'static str),
}

impl Display for PackageParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageParseError::UnknownCmd(cmd) => write!(f, "unknown command {cmd}"),
            PackageParseError::MissingArgs(args) => {
                write!(f, "insufficient args provided, expected [{args}]")
            }
        }
    }
}

#[macro_export]
macro_rules! package_enum {
    ($(#[$meta:meta])* $vis:vis enum $enum:ident {
        $(
            $var:ident ($cmd:expr $(=> $([$vec:ident])? $($($arg:ident),+ $(,)?)?)?)
        ),+ $(,)?
    }) => {
        $($($crate::check_exactly_one!("either one `[arg]` or multiple `args` may be specified", $($vec)? $(($($arg)+))?);)?)+
        $(#[$meta])*
        $vis enum $enum {
            $(
                $var $(($($crate::ignore!((Vec<String>) $vec))? $($($crate::ignore!((String) $arg)),+)?))?,
            )+
        }

        impl $enum {
            // needed because `Connection::send_package` takes
            // `impl Borrow<Package>`, so type inference can't
            // figure out `self.into()` should produce a `Package`
            pub fn package(self) -> $crate::connect::Package {
                self.into()
            }

            ::paste::paste! {$(
                pub fn [<$var:snake>]($($($vec: impl IntoIterator<Item = impl Into<String>>)? $($($arg: impl Into<String>),+)?)?) -> Self {
                    Self::$var $((
                        $($vec.into_iter().map(Into::into).collect())?
                        $($($arg.into()),+)?
                    ))?
                }
            )+}
        }

        impl ::std::convert::From<$enum> for $crate::connect::Package {
            fn from(value: $enum) -> Self {
                match value {
                    $($enum::$var $((
                        $($vec)?
                        $($($arg),+)?
                    ))? => $crate::connect::Package {
                        cmd: $cmd.to_string(),
                        $(args: $($vec)? $(vec![$($arg),+])? ,)?
                        ..Default::default()
                    },)+
                }
            }
        }

        impl ::std::convert::TryFrom<$crate::connect::Package> for $enum {
            type Error = $crate::connect::PackageParseError;

            fn try_from(value: $crate::connect::Package) -> Result<Self, Self::Error> {
                Ok(match value.cmd.as_str() {
                    $(
                        $cmd => {
                            $(
                                $(let [$($arg),+] = $crate::move_vec(value.args)
                                    .ok_or($crate::connect::PackageParseError::MissingArgs(stringify!($($arg),+)))?;)?
                                $(let $vec = value.args;)?
                            )?
                            Self::$var $(($($vec)? $($($arg),+)?))?
                        }
                    )+
                    _ => return Err($crate::connect::PackageParseError::UnknownCmd(value.cmd)),
                })
            }
        }
    };
}

#[macro_export]
macro_rules! ignore {
    (($($first:tt)*) $($_:tt)*) => {
        $($first)*
    };
}

#[macro_export]
macro_rules! check_exactly_one {
    ($_:expr, $tt:tt) => {};
    ($err:expr, $($_:tt)*) => {
        std::compile_error!($err);
    };
}

#[cfg(test)]
mod test {
    // just to test it compiles
    package_enum! {
        #[derive(Debug)]
        pub enum Test {
            NoOp("noop"),
            V("v" => [vec]),
            Args("args" => arg1, arg2),
        }
    }
}

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
