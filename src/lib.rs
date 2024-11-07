mod client;
mod connect;
mod requests;
mod response;
mod server;

pub use client::{PrimaryClient, SecondaryClient, TrivialClient};
pub use server::Server;

pub const SERVER_PORT: u16 = 6447;

/// move `N` strings out of a `Vec`
pub fn move_vec<const N: usize>(vec: Vec<String>) -> Option<[String; N]> {
    const EMPTY_STRING: String = String::new();
    let mut ret = [EMPTY_STRING; N];
    let mut iter = vec.into_iter();
    for item in ret.iter_mut().take(N) {
        *item = iter.next()?;
    }
    Some(ret)
}
