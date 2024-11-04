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
