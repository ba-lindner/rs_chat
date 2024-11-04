use std::io::{stdin, stdout, Write};

use crate::{client::inp_to_package, response::Response, Connection, Request};

use super::ClientErr;

/*
<msg> -> post "" <msg>
@<name> <msg> -> send <name> <msg>
/<channel> <msg> -> post <channel> <msg>

? -> this help
:? -> about + features
:q -> !quit
:w <channel>? -> names <channel>?
:c -> list_channels (also show joined ones)
:c <channel> <passwd>? -> new_channel / subscribe / unsubscribe
:b <name> -> (un)block
:o -> offenses
:f <name> -> forgive <name>
*/

pub struct SecondaryClient {
    conn: Connection,
    channels: Vec<String>,
    blocked: Vec<String>,
}

impl SecondaryClient {
    pub fn connect(port: u16) -> Result<Self, ClientErr> {
        Ok(Self {
            conn: Connection::to(("127.0.0.1", port))?,
            channels: Vec::new(),
            blocked: Vec::new(),
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

    fn parse_input(&self, inp: &str) -> Request {
        todo!()
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
