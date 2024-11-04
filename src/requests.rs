use crate::{move_vec, Package};

pub enum Request {
    Login(String),
    Listen,
    Ping,
    Post(String, String),
    Send(String, String),
    Names(String),
    About,
    Features,
    NewChannel(String, String),
    ListChannels,
    Subscribe(String, String),
    Unsubscribe(String),
    Block(String),
    Unblock(String),
    Offenses,
    Pardon(String),
}

pub enum RequestErr {
    Unknown(String),
    MissingArgs(&'static str),
    InvalidIdent,
}

impl Request {
    pub fn parse(pkg: Package) -> Result<Self, RequestErr> {
        fn get_args<const N: usize>(args: Vec<String>, expect: &'static str) -> Result<[String; N], RequestErr> {
            move_vec(args).ok_or(RequestErr::MissingArgs(expect))
        }
        Ok(match pkg.cmd.as_str() {
            "login" => {
                let [name] = get_args(pkg.args, "name")?;
                Request::Login(check_ident(name)?)
            }
            "listen" => Request::Listen,
            "ping" => Request::Ping,
            "post" => {
                let [channel, message] = get_args(pkg.args, "channel, message")?;
                Request::Post(check_ident(channel)?, message)
            }
            "send" => {
                let [name, message] = get_args(pkg.args, "name, message")?;
                Request::Send(check_ident(name)?, message)
            }
            "names" => {
                let [channel] = get_args(pkg.args, "channel")?;
                Request::Names(check_ident(channel)?)
            }
            "about" => Request::About,
            "features" => Request::Features,
            "new_channel" => {
                let [channel, password] = get_args(pkg.args, "channel, password")?;
                Request::NewChannel(check_ident(channel)?, password)
            }
            "list_channels" => Request::ListChannels,
            "subscribe" => {
                let [channel, password] = get_args(pkg.args, "channel, password")?;
                Request::Subscribe(check_ident(channel)?, password)
            }
            "unsubscribe" => {
                let [channel] = get_args(pkg.args, "channel")?;
                Request::Unsubscribe(check_ident(channel)?)
            }
            "block" => {
                let [name] = get_args(pkg.args, "name")?;
                Request::Block(check_ident(name)?)
            }
            "unblock" => {
                let [name] = get_args(pkg.args, "name")?;
                Request::Unblock(check_ident(name)?)
            }
            "offenses" => Request::Offenses,
            "pardon" => {
                let [name] = get_args(pkg.args, "name")?;
                Request::Pardon(check_ident(name)?)
            }
            _ => return Err(RequestErr::Unknown(pkg.cmd)),
        })
    }

    pub fn to_package(self) -> Package {
        match self {
            Request::Login(name) => Package { cmd: "login".to_string(), args: vec![name] },
            Request::Listen => Package { cmd: "listen".to_string(), args: Vec::new() },
            Request::Ping => Package { cmd: "ping".to_string(), args: Vec::new() },
            Request::Post(channel, msg) => Package { cmd: "post".to_string(), args: vec![channel, msg] },
            Request::Send(name, msg) => Package { cmd: "send".to_string(), args: vec![name, msg] },
            Request::Names(channel) => Package { cmd: "names".to_string(), args: vec![channel] },
            Request::About => Package { cmd: "about".to_string(), args: Vec::new() },
            Request::Features => Package { cmd: "features".to_string(), args: Vec::new() },
            Request::NewChannel(channel, passwd) => Package { cmd: "new_channel".to_string(), args: vec![channel, passwd] },
            Request::ListChannels => Package { cmd: "list_channels".to_string(), args: Vec::new() },
            Request::Subscribe(channel, passwd) => Package { cmd: "subscribe".to_string(), args: vec![channel, passwd] },
            Request::Unsubscribe(channel) => Package { cmd: "unsubscribe".to_string(), args: vec![channel] },
            Request::Block(name) => Package { cmd: "block".to_string(), args: vec![name] },
            Request::Unblock(name) => Package { cmd: "unblock".to_string(), args: vec![name] },
            Request::Offenses => Package { cmd: "offenses".to_string(), args: Vec::new() },
            Request::Pardon(name) => Package { cmd: "pardon".to_string(), args: vec![name] },
        }
    }
}

fn check_ident(ident: String) -> Result<String, RequestErr> {
    ident
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
        .then_some(ident)
        .ok_or(RequestErr::InvalidIdent)
}

impl RequestErr {
    pub fn get_package(self) -> Package {
        match self {
            RequestErr::Unknown(cmd) => Package::err(format!("unknown command '{cmd}'")),
            RequestErr::MissingArgs(args) => Package::err(format!("please provide {args}")),
            RequestErr::InvalidIdent => Package::err("invalid name"),
        }
    }
}
