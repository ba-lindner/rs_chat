use crate::package_enum;

package_enum! {
    /// Responses the server sends to a client
    ///
    /// Responses can be categorized into two groups:
    /// * `Ack`, `Err` and `Info` are synchronous responses to
    /// [`Request`](crate::requests::Request)s sent by clients.
    /// Each client request will result in EXACTLY one such
    /// response. To identify corresponding pairs, the responses
    /// are always sent in the order the requests are received.
    /// * `Msg` is an asynchronous response to a request made by
    /// another client (namely, the request to send you a message).
    /// A server may send any number of these responses at
    /// any time to any client, regardless of any other
    /// communication they might have at the moment.
    pub enum Response {
        /// ACK: request successfull, no further data needed
        Ack("ack"),
        /// ERR: request unsuccessfull, reason as argument
        Err("err" => why),
        /// INFO: request successfull, requested data provided as args
        Info("info" => [data]),
        /// MSG: you've got mail!
        Msg("msg" => channel, name, msg),
    }
}

impl Response {
    /// should this response be counted towards a clients offenses?
    pub fn is_bad(&self) -> bool {
        matches!(self, Self::Err { .. })
    }
}
