use std::{
    collections::HashMap,
    io::Error,
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use crate::{Connection, Package};

struct Client {
    conn: Connection,
    name: Option<String>,
}

impl Client {
    pub fn ack(&mut self) {
        self.conn.send_package(&*Package::ACK);
    }

    pub fn err(&mut self, why: impl Into<String>) {
        self.conn.send_package(Package::err(why));
    }

    pub fn info(&mut self, data: impl IntoIterator<Item = impl Into<String>>) {
        self.conn.send_package(Package {
            cmd: "info".to_string(),
            args: data.into_iter().map(Into::into).collect(),
        });
    }
}

struct Channel {
    name: String,
    password: String,
    members: Vec<String>,
    msg_queue: Vec<Package>,
}

pub struct Server {
    login_rx: Receiver<Client>,
    active_clients: HashMap<String, Client>,
    passive_clients: Vec<Client>,
    channels: HashMap<String, Channel>,
}

impl Server {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        login_thread(tx);
        Self {
            login_rx: rx,
            active_clients: HashMap::new(),
            passive_clients: Vec::new(),
            channels: HashMap::new(),
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.collect_new_clients();
            self.handle_request();
            self.prune();
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn collect_new_clients(&mut self) {
        while let Ok(mut new_client) = self.login_rx.try_recv() {
            if let Some(name) = &new_client.name {
                if self.active_clients.contains_key(name) {
                    new_client.err("name already used");
                } else {
                    let name = name.clone();
                    new_client.ack();
                    self.active_clients.insert(name, new_client);
                }
            } else {
                new_client.ack();
                self.passive_clients.push(new_client);
            }
        }
    }

    fn handle_request(&mut self) {
        let mut posted_msg: Vec<Package> = Vec::new();
        let mut direct_msg: Vec<(String, Package)> = Vec::new();
        let mut name_req: Vec<String> = Vec::new();
        for (name, client) in &mut self.active_clients {
            while let Some(pkg) = client.conn.get_package() {
                match pkg.cmd.as_str() {
                    "ping" => client.ack(),
                    "post" => {
                        if pkg.args.is_empty() {
                            client.err("please provide a message");
                        } else if let Some(channel) = pkg.args.get(1) {
                            if let Some(channel) = self.channels.get_mut(channel) {
                                if channel.members.contains(name) {
                                    channel.msg_queue.push(Package::msg(
                                        &channel.name,
                                        name,
                                        &pkg.args[0],
                                    ));
                                } else {
                                    client.err("not subscribed to channel");
                                }
                            } else {
                                client.err("channel doesn't exist");
                            }
                        } else {
                            posted_msg.push(Package::msg("", name, &pkg.args[0]));
                        }
                    }
                    "send" => {
                        if let [recv, msg] = pkg.args.as_slice() {
                            direct_msg.push((recv.clone(), Package::msg("__direct", name, msg)));
                            client.ack();
                        } else {
                            client.err("please provide receiver and message");
                        }
                    }
                    "names" => {
                        name_req.push(name.clone());
                    }
                    "about" => {
                        client.info([format!("rs_chat server v{}", env!("CARGO_PKG_NAME"))]);
                    }
                    "new_channel" => {
                        if let [channel, passwd] = pkg.args.as_slice() {
                            if channel == "__direct" || self.channels.contains_key(channel) {
                                client.err("channel name already used");
                            } else {
                                self.channels.insert(
                                    channel.clone(),
                                    Channel {
                                        name: channel.clone(),
                                        password: passwd.clone(),
                                        members: vec![name.clone()],
                                        msg_queue: Vec::new(),
                                    },
                                );
                            }
                        } else {
                            client.err("please provide channel name and a password");
                        }
                    }
                    "list_channels" => {
                        client.info(self.channels.keys());
                    }
                    "subscribe" => {
                        if let [channel, passwd] = pkg.args.as_slice() {
                            if let Some(ch) = self.channels.get_mut(channel) {
                                if &ch.password == passwd {
                                    ch.members.push(name.clone());
                                    client.ack();
                                } else {
                                    client.err("wrong password");
                                }
                            } else {
                                client.err("channel doesn't exist");
                            }
                        } else {
                            client.err("please provide channel and password");
                        }
                    }
                    "unsubscribe" => {
                        if let Some(channel) = pkg.args.get(0) {
                            if let Some(ch) = self.channels.get_mut(channel) {
                                if let Some(idx) = ch.members.iter().position(|n| n == name) {
                                    ch.members.swap_remove(idx);
                                    client.ack();
                                } else {
                                    client.err("not subscribed to channel");
                                }
                            } else {
                                client.err("channel doesn't exist");
                            }
                        } else {
                            client.err("please provide the channel");
                        };
                    }
                    cmd => {
                        client.err(format!("unknown command {cmd}"));
                    }
                }
            }
        }
        for msg in posted_msg {
            for client in self.active_clients.values_mut() {
                client.conn.send_package(&msg);
            }
            for client in &mut self.passive_clients {
                client.conn.send_package(&msg);
            }
        }
        for (recipient, msg) in direct_msg {
            if let Some(recp) = self.active_clients.get_mut(&recipient) {
                recp.conn.send_package(msg);
            }
        }
        if !name_req.is_empty() {
            let name_pkg = Package {
                cmd: "info".to_string(),
                args: self.active_clients.keys().map(Clone::clone).collect(),
            };
            for name in name_req {
                if let Some(client) = self.active_clients.get_mut(&name) {
                    client.conn.send_package(&name_pkg);
                }
            }
        }
    }

    fn prune(&mut self) {
        self.active_clients.retain(|_, c| c.conn.alive());
        self.passive_clients.retain(|c| c.conn.alive());
        self.channels.retain(|_, c| !c.members.is_empty());
    }
}

fn login_thread(tx: Sender<Client>) {
    thread::spawn(move || {
        let listener = TcpListener::bind("0.0.0.0:6447").expect("failed to bind to port");
        for stream in listener.incoming() {
            if let Some(client) = login_client(stream) {
                tx.send(client).unwrap();
            }
        }
    });
}

fn login_client(stream: Result<TcpStream, Error>) -> Option<Client> {
    let mut conn = Connection::new(stream.ok()?)?;
    for _ in 0..10 {
        if let Some(mut pkg) = conn.get_package() {
            match (pkg.cmd.as_str(), pkg.args.len()) {
                ("login", 1) => {
                    let name = std::mem::take(&mut pkg.args[0]);
                    if name.is_empty() {
                        conn.send_package(Package::err("please provide a name"));
                        continue;
                    }
                    let client = Client {
                        conn,
                        name: Some(name),
                    };
                    return Some(client);
                }
                ("listen", 0) => {}
                _ => {
                    conn.send_package(Package::err("please login first"));
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
    None
}
