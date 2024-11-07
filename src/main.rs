use clap::{Parser, Subcommand};
use rs_chat::{PrimaryClient, SecondaryClient, Server, TrivialClient};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a server
    Server,
    /// Start the primary client
    Read {
        /// connection string
        ///
        /// must be in the format `name@address`,
        /// e.g. `me@localhost`
        conn: String,
    },
    /// Start the secondary client
    Write {
        /// the port the primary client is listening on
        port: u16,
    },
    /// trivial client for testing purposes only
    Test {
        /// connection string
        conn: String,
    },
}

fn main() {
    match Cli::parse().command {
        Commands::Server => Server::new().unwrap().run(),
        Commands::Read { conn } => {
            let (name, addr) = conn_str(&conn);
            PrimaryClient::connect(addr, name).unwrap().run();
        }
        Commands::Write { port } => SecondaryClient::connect(port).unwrap().run(),
        Commands::Test { conn } => {
            let (name, addr) = conn_str(&conn);
            TrivialClient::connect(addr, name).unwrap().run();
        }
    }
}

fn conn_str(conn: &str) -> (&str, &str) {
    conn.split_once('@').unwrap_or_else(|| {
        eprintln!("please provide username and server address in the format name@address");
        std::process::exit(1);
    })
}

#[cfg(test)]
mod test {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli() {
        Cli::command().debug_assert();
    }
}
