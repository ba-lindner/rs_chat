use crate::{
    package::{Package, PackageParseError},
    package_enum,
    response::Response,
};

package_enum! {
    /// Requests a client can make to a server
    /// 
    /// Each such request will be responded to with EXACTLY one of
    /// the synchronous [`Response`]s.
    pub enum Request {
        /// Log into a server with the given name
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// For this request, the response is guaranteed to be sent
        /// BEFORE any `Msg` response is sent to the client.
        /// 
        /// ## Error cases
        /// * the client is already logged on
        /// * the name is rejected by the server
        /// * the name is already used by another client
        Login("login" => name),
        /// Listen to the global channel of a server
        /// 
        /// This is the alternate login method available.
        /// A client logged in via `Listen` is considered a
        /// 'passive' client, meaning any requests made by the client
        /// will be silently ignored by the server.
        /// 
        /// This request is responded to with `Ack`.
        /// 
        /// For this request, the response is guaranteed to be sent
        /// BEFORE any `Msg` response is sent to the client.
        /// 
        /// This request will never fail.
        Listen("listen"),
        /// Check if the connection is still ok
        /// 
        /// This request is responded to with `Ack`.
        /// 
        /// This request will never fail.
        Ping("ping"),
        /// Post a message to a channel
        /// 
        /// To post a message to the global channel,
        /// simply leave the channel argument empty
        /// (as the name of the global channel is the
        /// empty string).
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the channel does not exist
        /// * you have not joined the channel
        Post("post" => channel, msg),
        /// Send a message to another user
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the user does not exist
        /// * the user has blocked you
        /// * you have blocked the user
        Send("send" => name, msg),
        /// Get a list of the users that have joined a channel
        /// 
        /// To get a list of users for the global channel,
        /// simply leave the channel argument empty
        /// (as the name of the global channel is the
        /// empty string).
        /// Note that this will not list every user as clients
        /// are allowed to unsubscribe from the global channel.
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the channel does not exist
        /// * you have not joined the channel
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

    /// Check whether the request contains invalid identifiers
    /// 
    /// Currently, this uses [`is_ident_ok`] to check whether
    /// a client name or channel name is invalid.
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
