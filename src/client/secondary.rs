use std::io::{stdin, stdout, Write};

use crate::{client::inp_to_package, response::Response, Connection};

use super::ClientErr;

pub struct SecondaryClient {
    conn: Connection,
}

impl SecondaryClient {
    pub fn connect(port: u16) -> Result<Self, ClientErr> {
        Ok(Self {
            conn: Connection::to(("127.0.0.1", port))?,
        })
    }

    pub fn run(&mut self) {
        println!("rs_chat secondary client v{}", env!("CARGO_PKG_VERSION"));
        loop {
            print!("> ");
            stdout().flush().unwrap();
            let mut inp = String::new();
            stdin().read_line(&mut inp).unwrap();
            self.conn.send_package(inp_to_package(inp.trim_end()));
            let Some(answer) = self.conn.wait_package() else {
                eprintln!("disconnected");
                return;
            };
            match answer.try_into() {
                Ok(resp) => Self::print_answer(resp),
                Err(why) => eprintln!("server sent invalid response: {why:?}"),
            }
        }
    }

    fn print_answer(answer: Response) {
        match answer {
            Response::Ack => println!("request succeeded"),
            Response::Err(why) => eprintln!("request failed: {why}"),
            Response::Info(data) => {
                println!(
                    "received data: {}",
                    data.into_iter()
                        .reduce(|acc, v| acc + ", " + &v)
                        .unwrap_or_default()
                );
            }
            Response::Msg(_, _, _) => eprintln!("got a message. this shouldn't happen."),
        }
    }
}
