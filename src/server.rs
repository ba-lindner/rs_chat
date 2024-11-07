use std::{
    collections::{HashMap, HashSet},
    io::Error,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

//mod bot;
mod login;

use crate::{connection::Connection, package::Package, requests::Request, response::Response};

pub const GLOBAL_CHANNEL_NAME: &str = "";
pub const DIRECT_CHANNEL_NAME: &str = "__direct";
const MAX_OFFENSES: u8 = 5;

struct Client {
    conn: Connection,
    name: Option<String>,
    offenses: u8,
    blocked: HashSet<String>,
}

impl Client {
    pub fn new(conn: Connection, name: Option<String>) -> Self {
        Self {
            conn,
            name,
            offenses: 0,
            blocked: HashSet::new(),
        }
    }

    pub fn should_remain(&self) -> bool {
        if let Some(name) = &self.name {
            if self.offenses >= MAX_OFFENSES {
                println!("{name} was kicked");
            }
            if !self.conn.alive() {
                println!("{name} left");
            }
        }
        self.conn.alive() && self.offenses < MAX_OFFENSES
    }
}

#[derive(Default)]
struct Channel {
    name: String,
    password: String,
    members: Vec<String>,
    msg_queue: Vec<Package>,
}

impl Channel {
    pub fn new(name: String, password: String, founder: String) -> Self {
        Self {
            name,
            password,
            members: vec![founder],
            msg_queue: Vec::new(),
        }
    }

    pub fn append_msg(&mut self, from: String, msg: String) {
        self.msg_queue
            .push(Response::msg(self.name.clone(), from, msg).package());
    }
}

pub struct Server {
    login_rx: Receiver<Client>,
    active_clients: HashMap<String, Client>,
    passive_clients: Vec<Client>,
    channels: HashMap<String, Channel>,
}

impl Server {
    pub const ABOUT: &'static str = concat!(
        "rs_chat server v",
        env!("CARGO_PKG_VERSION"),
        " by blindner"
    );

    pub const FEATURES: [&'static str; 4] = ["basic", "direct", "channels", "offenses"];

    pub fn new() -> Result<Self, Error> {
        let (tx, rx) = mpsc::channel();
        login::login_thread(tx)?;
        Ok(Self {
            login_rx: rx,
            active_clients: HashMap::new(),
            passive_clients: Vec::new(),
            channels: HashMap::from([(String::new(), Channel::default())]),
        })
    }

    pub fn run(&mut self) -> ! {
        println!("{}", Self::ABOUT);
        loop {
            self.collect_new_clients();
            for (client, req) in self.collect_requests() {
                let (Ok(resp) | Err(resp)) = self.respond_to(&client, req);
                if let Some(client) = self.active_clients.get_mut(&client) {
                    if resp.is_bad() {
                        client.offenses += 1;
                    }
                    client.conn.send_package(resp.package());
                }
            }
            self.send_queues();
            self.prune();
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn collect_new_clients(&mut self) {
        while let Ok(mut new_client) = self.login_rx.try_recv() {
            if let Some(name) = &new_client.name {
                if self.active_clients.contains_key(name) {
                    new_client
                        .conn
                        .send_package(Response::err("name already used").package());
                } else {
                    let name = name.clone();
                    println!("{name} has joined");
                    new_client.conn.send_package(Response::Ack.package());
                    self.active_clients.insert(name.clone(), new_client);
                    self.channels
                        .get_mut(GLOBAL_CHANNEL_NAME)
                        .expect("global channel should always exist")
                        .members
                        .push(name);
                }
            } else {
                new_client.conn.send_package(Response::Ack.package());
                self.passive_clients.push(new_client);
            }
        }
    }

    fn collect_requests(&mut self) -> Vec<(String, Request)> {
        let mut collected = Vec::new();
        for (name, client) in &mut self.active_clients {
            while let Some(pkg) = client.conn.get_package() {
                match Request::parse(pkg) {
                    Ok(req) => collected.push((name.clone(), req)),
                    Err(why) => {
                        client.offenses += 1;
                        client.conn.send_package(why.package())
                    }
                }
            }
        }
        collected
    }

    pub fn respond_to(&mut self, client: &String, req: Request) -> Result<Response, Response> {
        Ok(match req {
            Request::Login(_) | Request::Listen => Response::err("already logged in"),
            Request::Ping => Response::Ack,
            Request::Post(channel, msg) => {
                self.get_channel(client, &channel)?
                    .append_msg(client.clone(), msg);
                Response::Ack
            }
            Request::Send(to, msg) => {
                if self.get_client(client)?.blocked.contains(&to) {
                    Response::err("user was blocked")
                } else {
                    let cl = self.get_client(&to)?;
                    if cl.blocked.contains(client) {
                        Response::err("you were blocked by user")
                    } else {
                        cl.conn.send_package(
                            Response::msg(DIRECT_CHANNEL_NAME, client.clone(), msg).package(),
                        );
                        Response::Ack
                    }
                }
            }
            Request::Names(channel) => Response::info(&self.get_channel(client, &channel)?.members),
            Request::About => Response::info([Self::ABOUT]),
            Request::Features => Response::info(Self::FEATURES),
            Request::NewChannel(channel, passwd) => {
                if self.channels.contains_key(&channel) || channel == DIRECT_CHANNEL_NAME {
                    Response::err("channel exists already")
                } else {
                    self.channels.insert(
                        channel.clone(),
                        Channel::new(channel, passwd, client.clone()),
                    );
                    Response::Ack
                }
            }
            Request::ListChannels => Response::info(self.channels.keys()),
            Request::Subscribe(channel, passwd) => {
                let chan = self
                    .channels
                    .get_mut(&channel)
                    .ok_or(Response::err("channel doesn't exist"))?;
                if chan.password == passwd {
                    chan.members.push(client.clone());
                    Response::Ack
                } else {
                    Response::err("wrong password")
                }
            }
            Request::Unsubscribe(channel) => {
                self.get_channel(client, &channel)?
                    .members
                    .retain(|n| n != client);
                Response::Ack
            }
            Request::Block(name) => {
                self.get_client(&name)?;
                if self.get_client(client)?.blocked.insert(name) {
                    Response::Ack
                } else {
                    Response::err("user already blocked")
                }
            }
            Request::Unblock(name) => {
                if self.get_client(client)?.blocked.remove(&name) {
                    Response::Ack
                } else {
                    Response::err("user wasn't blocked")
                }
            }
            Request::Offenses => Response::info([
                self.get_client(client)?.offenses.to_string(),
                MAX_OFFENSES.to_string(),
            ]),
            Request::Pardon(name) => {
                let cl = self.get_client(&name)?;
                if cl.offenses > 0 {
                    cl.offenses -= 1;
                    Response::Ack
                } else {
                    Response::err("user has no offenses")
                }
            }
        })
    }

    fn get_channel(&mut self, client: &String, channel: &String) -> Result<&mut Channel, Response> {
        let chan = self
            .channels
            .get_mut(channel)
            .ok_or(Response::err("channel doesn't exist"))?;
        chan.members
            .contains(client)
            .then_some(chan)
            .ok_or(Response::err("not subscribed to channel"))
    }

    fn get_client(&mut self, client: &String) -> Result<&mut Client, Response> {
        self.active_clients
            .get_mut(client)
            .ok_or(Response::err("user doesn't exist"))
    }

    fn send_queues(&mut self) {
        for channel in self.channels.values_mut() {
            for msg in channel.msg_queue.drain(..) {
                for name in &channel.members {
                    if let Some(client) = self.active_clients.get_mut(name) {
                        client.conn.send_package(&msg);
                    }
                }
                if channel.name == GLOBAL_CHANNEL_NAME {
                    for client in &mut self.passive_clients {
                        client.conn.send_package(&msg);
                    }
                }
            }
        }
    }

    fn prune(&mut self) {
        self.active_clients.retain(|_, c| c.should_remain());
        self.passive_clients.retain(|c| c.conn.alive());
        self.channels.retain(|_, c| {
            c.members.retain(|n| self.active_clients.contains_key(n));
            c.name == GLOBAL_CHANNEL_NAME || !c.members.is_empty()
        });
    }
}
