use crate::Package;

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
        Ok(match pkg.cmd.as_str() {
            "login" => Request::Login(check_ident(one_arg(pkg.args, "name")?)?),
            "listen" => Request::Listen,
            "ping" => Request::Ping,
            "post" => {
                let (channel, message) = two_args(pkg.args, "channel, message")?;
                Request::Post(check_ident(channel)?, message)
            }
            "send" => {
                let (name, message) = two_args(pkg.args, "name, message")?;
                Request::Send(check_ident(name)?, message)
            }
            "names" => Request::Names(check_ident(one_arg(pkg.args, "channel")?)?),
            "about" => Request::About,
            "features" => Request::Features,
            "new_channel" => {
                let (channel, password) = two_args(pkg.args, "channel, password")?;
                Request::NewChannel(check_ident(channel)?, password)
            }
            "list_channels" => Request::ListChannels,
            "subscribe" => {
                let (channel, password) = two_args(pkg.args, "channel, password")?;
                Request::Subscribe(check_ident(channel)?, password)
            }
            "unsubscribe" => Request::Unsubscribe(check_ident(one_arg(pkg.args, "channel")?)?),
            "block" => Request::Block(check_ident(one_arg(pkg.args, "name")?)?),
            "unblock" => Request::Unblock(check_ident(one_arg(pkg.args, "name")?)?),
            "offenses" => Request::Offenses,
            "pardon" => Request::Pardon(check_ident(one_arg(pkg.args, "name")?)?),
            _ => return Err(RequestErr::Unknown(pkg.cmd)),
        })
    }
}

fn one_arg(args: Vec<String>, expect: &'static str) -> Result<String, RequestErr> {
    args.into_iter()
        .next()
        .ok_or(RequestErr::MissingArgs(expect))
}

fn two_args(args: Vec<String>, expect: &'static str) -> Result<(String, String), RequestErr> {
    let mut args = args.into_iter();
    Ok((
        args.next().ok_or(RequestErr::MissingArgs(expect))?,
        args.next().ok_or(RequestErr::MissingArgs(expect))?,
    ))
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
