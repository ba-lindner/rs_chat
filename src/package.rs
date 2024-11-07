use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Package {
    pub cmd: String,
    pub args: Vec<String>,
}

impl Package {
    pub const PKG_START: &'static str = "\x02"; // STX
    pub const CMD_END: &'static str = "\x16"; // SYN
    pub const ARG_END: &'static str = "\x19"; // EM
    pub const PKG_END: &'static str = "\x03"; // ETX

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
            $(#[$doc:meta])* $var:ident ($cmd:expr $(=> $([$vec:ident])? $($($arg:ident),+ $(,)?)?)?)
        ),+ $(,)?
    }) => {
        $($($crate::check_exactly_one!("either one `[arg]` or multiple `args` may be specified", $($vec)? $(($($arg)+))?);)?)+
        $(#[$meta])*
        $vis enum $enum {
            $(
                $(#[$doc])* $var $((
                    $($crate::ignore!((Vec<String>) $vec))?
                    $($($crate::ignore!((String) $arg)),+)?
                ))?,
            )+
        }

        impl $enum {
            // needed because `Connection::send_package` takes
            // `impl Borrow<Package>`, so type inference can't
            // figure out `self.into()` should produce a `Package`
            pub fn package(self) -> $crate::package::Package {
                self.into()
            }

            ::paste::paste! {$($(
                pub fn [<$var:snake>]($($vec: impl IntoIterator<Item = impl Into<String>>)? $($($arg: impl Into<String>),+)?) -> Self {
                    Self::$var (
                        $($vec.into_iter().map(Into::into).collect())?
                        $($($arg.into()),+)?
                    )
                }
            )?)+}
        }

        impl ::std::convert::From<$enum> for $crate::package::Package {
            fn from(value: $enum) -> Self {
                match value {
                    $($enum::$var $((
                        $($vec)?
                        $($($arg),+)?
                    ))? => $crate::package::Package {
                        cmd: $cmd.to_string(),
                        $(args: $($vec)? $(vec![$($arg),+])? ,)?
                        ..Default::default()
                    },)+
                }
            }
        }

        impl ::std::convert::TryFrom<$crate::package::Package> for $enum {
            type Error = $crate::package::PackageParseError;

            fn try_from(value: $crate::package::Package) -> Result<Self, Self::Error> {
                Ok(match value.cmd.as_str() {
                    $(
                        $cmd => {
                            $(
                                $(let [$($arg),+] = $crate::move_vec(value.args)
                                    .ok_or($crate::package::PackageParseError::MissingArgs(stringify!($($arg),+)))?;)?
                                $(let $vec = value.args;)?
                            )?
                            Self::$var $(($($vec)? $($($arg),+)?))?
                        }
                    )+
                    _ => return Err($crate::package::PackageParseError::UnknownCmd(value.cmd)),
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
