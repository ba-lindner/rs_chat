mod client;
mod connect;
mod response;
mod server;

pub use client::{TrivialClient, PrimaryClient, SecondaryClient};
pub use connect::{Connection, Package};
pub use server::Server;

pub const SERVER_PORT: u16 = 6447;
