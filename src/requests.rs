use crate::{
    connect::{Package, PackageParseError},
    package_enum,
    response::Response,
};

package_enum! {
    pub enum Request {
        Login("login" => name),
        Listen("listen"),
        Ping("ping"),
        Post("post" => channel, msg),
        Send("send" => name, msg),
        Names("names" => channel),
        About("about"),
        Features("features"),
        NewChannel("new_channel" => channel, password),
        ListChannels("list_channels"),
        Subscribe("subscribe" => channel, password),
        Unsubscribe("unsubscribe" => channel),
        Block("block" => name),
        Unblock("unblock" => name),
        Offenses("offenses"),
        Pardon("pardon" => name),
    }
}

impl Request {
    /// parse and validate a [`Request`]
    pub fn parse(pkg: Package) -> Result<Self, RequestErr> {
        let res = Self::try_from(pkg)?;
        res.check_idents()?;
        Ok(res)
    }

    pub fn check_idents(&self) -> Result<(), RequestErr> {
        match self {
            Request::Login(name)
            | Request::Send(name, _)
            | Request::Block(name)
            | Request::Unblock(name)
            | Request::Pardon(name) => is_ident_ok(name)
                .then_some(())
                .ok_or(RequestErr::InvalidName),
            Request::Post(channel, _)
            | Request::Names(channel)
            | Request::NewChannel(channel, _)
            | Request::Subscribe(channel, _)
            | Request::Unsubscribe(channel) => is_ident_ok(channel)
                .then_some(())
                .ok_or(RequestErr::InvalidName),
            _ => Ok(()),
        }
    }
}

fn is_ident_ok(ident: &str) -> bool {
    ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Debug)]
pub enum RequestErr {
    InvalidName,
    InvalidChannel,
    ParseErr(PackageParseError),
}

impl RequestErr {
    pub fn package(self) -> Package {
        Response::from(self).package()
    }
}

impl From<RequestErr> for Response {
    fn from(value: RequestErr) -> Self {
        match value {
            RequestErr::InvalidName => Self::err("invalid name"),
            RequestErr::InvalidChannel => Self::err("invalid channel"),
            RequestErr::ParseErr(err) => Self::Err(format!("{err}")),
        }
    }
}

impl From<PackageParseError> for RequestErr {
    fn from(value: PackageParseError) -> Self {
        Self::ParseErr(value)
    }
}
