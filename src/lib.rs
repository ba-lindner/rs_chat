mod client;
mod connect;
mod requests;
mod response;
mod server;

pub use client::{TrivialClient, PrimaryClient, SecondaryClient};
pub use connect::{Connection, Package};
pub use requests::{Request, RequestErr};
pub use response::{Response, ResponseError};
pub use server::Server;

pub const SERVER_PORT: u16 = 6447;

/// move `N` strings out of a `Vec`
pub fn move_vec<const N: usize>(args: Vec<String>) -> Option<[String; N]> {
    const EMPTY_STRING: String = String::new();
    let mut ret = [EMPTY_STRING; N];
    let mut args = args.into_iter();
    for i in 0..N {
        ret[i] = args.next()?;
    }
    Some(ret)
}
