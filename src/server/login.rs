use std::{io::Error, net::TcpListener, sync::mpsc::Sender, thread, time::Duration};

use crate::{connection::Connection, requests::Request, response::Response, SERVER_PORT};

use super::Client;

const MAX_AGE: u32 = 200;

pub fn login_thread(tx: Sender<Client>) -> Result<(), Error> {
    let listener = TcpListener::bind(("127.0.0.1", SERVER_PORT))?;
    listener.set_nonblocking(true)?;
    thread::spawn(move || {
        let mut incoming: Vec<(Connection, u32)> = Vec::new();
        loop {
            while let Ok((stream, _)) = listener.accept() {
                if let Ok(conn) = Connection::new(stream) {
                    incoming.push((conn, 0));
                }
            }
            incoming = incoming
                .into_iter()
                .flat_map(|(mut conn, age)| match try_login(&mut conn) {
                    Ok(name) => {
                        tx.send(Client::new(conn, name)).expect("server died");
                        None
                    }
                    Err(()) => (age < MAX_AGE).then_some((conn, age + 1)),
                })
                .collect();
            thread::sleep(Duration::from_millis(50));
        }
    });
    Ok(())
}

fn try_login(conn: &mut Connection) -> Result<Option<String>, ()> {
    if let Some(pkg) = conn.get_package() {
        match pkg.try_into() {
            Ok(Request::Login(name)) => {
                if name.is_empty() {
                    conn.send_package(Response::err("please provide a name").package());
                } else {
                    return Ok(Some(name));
                }
            }
            Ok(Request::Listen) => {
                return Ok(None);
            }
            _ => {
                conn.send_package(Response::err("please login first").package());
            }
        }
    }
    Err(())
}
