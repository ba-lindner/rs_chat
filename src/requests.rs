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
        /// This request is responded to with `Info` in case of success.
        /// The response contains a list with the
        /// names of all clients in this channel.
        /// 
        /// ## Error cases
        /// * the channel does not exist
        /// * you have not joined the channel
        Names("names" => channel),
        /// Get the name of the server
        /// 
        /// This request is responded to with `Info` in case of success.
        /// The response contains AT LEAST one argument with the name
        /// of the server. A server MAY send any number of additional arguments.
        /// 
        /// This request will never fail.
        About("about"),
        /// Get the features supported by the server
        /// 
        /// This request is responded to with `Info` in case of success.
        /// The response contains a list of all supported features.
        /// 
        /// This request will never fail.
        Features("features"),
        /// Create a new channel
        /// 
        /// To create a new channel that anyone can join,
        /// simply leave the password empty.
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the channel name is invalid
        /// * the channel name is already used
        NewChannel("new_channel" => channel, password),
        /// List all available channels
        /// 
        /// This request is responded to with `Info` in case of success.
        /// The response contains a list of all channels.
        /// 
        /// This request will never fail.
        ListChannels("list_channels"),
        /// Subscribe to a channel
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the channel name is invalid
        /// * the channel doesn't exist
        /// * the password is incorrect
        /// * you have already subscribed to the channel
        Subscribe("subscribe" => channel, password),
        /// Unsubscribe from a channel
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the channel name is invalid
        /// * the channel doesn't exist
        /// * you have not subscribed to the channel
        Unsubscribe("unsubscribe" => channel),
        /// Block direct messages from a user
        /// 
        /// This prevents any direct communication between the client
        /// sending this request and the blocked client.
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the user name is invalid
        /// * the user doesn't exist
        /// * you have already blocked the user
        Block("block" => name),
        /// Unblock a user
        /// 
        /// This reverts the effect of a prior `Block` request
        /// 
        /// Note that you may unblock a user even after he has left,
        /// when all other requests regarding that user would fail with
        /// 'user doesn't exist'.
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the user name is invalid
        /// * you didn't blocked the user
        Unblock("unblock" => name),
        /// Find out how often you have offended the server
        /// 
        /// This request is responded to with `Info` in case of success.
        /// The response contains AT LEAST one argument with the number
        /// of your offenses. A server MAY send an additional
        /// argument containing the maximal number of offenses.
        /// 
        /// This request will never fail.
        Offenses("offenses"),
        /// Pardon another player, reducing his offenses by one
        /// 
        /// This request is responded to with `Ack` in case of success.
        /// 
        /// ## Error cases
        /// * the user name is invalid
        /// * the user doesn't exist
        /// * the user did not have any offenses
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

/// Reasons a request is immediately rejected
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
