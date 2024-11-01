use std::{
    cell::LazyCell,
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

macro_rules! get_channel {
    ($channels:expr, $channel:expr, $client:expr) => {{
        if let Some(chan) = $channels.get_mut(&$channel) {
            if let Some(name) = &$client.name {
                if chan.members.contains(name) {
                    Some(chan)
                } else {
                    $client.err("not subscribed to channel");
                    None
                }
            } else {
                $client.err("not subscribed to channel");
                None
            }
        } else {
            $client.err("channel doesn't exist");
            None
        }
    }};
}

pub struct Server {
    login_rx: Receiver<Client>,
    active_clients: HashMap<String, Client>,
    passive_clients: Vec<Client>,
    channels: HashMap<String, Channel>,
    direct_msg: Vec<(String, Package)>,
}

impl Server {
    pub const ABOUT: LazyCell<String> =
        LazyCell::new(|| format!("rs_chat server v{}", env!("CARGO_PKG_VERSION")));

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        login_thread(tx);
        let global = Channel {
            name: String::new(),
            password: String::new(),
            members: Vec::new(),
            msg_queue: Vec::new(),
        };
        Self {
            login_rx: rx,
            active_clients: HashMap::new(),
            passive_clients: Vec::new(),
            channels: {
                let mut map = HashMap::new();
                map.insert(String::new(), global);
                map
            },
            direct_msg: Vec::new(),
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.collect_new_clients();
            self.handle_requests();
            self.flush_queues();
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
                    self.active_clients.insert(name.clone(), new_client);
                    self.channels.get_mut("").expect("global channel should always exist").members.push(name);
                }
            } else {
                new_client.ack();
                self.passive_clients.push(new_client);
            }
        }
    }

    fn handle_requests(&mut self) {
        for (name, client) in &mut self.active_clients {
            while let Some(pkg) = client.conn.get_package() {
                match Request::parse(pkg) {
                    Request::Login(_) | Request::Listen => client.err("already logged in"),
                    Request::Ping => client.ack(),
                    Request::Post(channel, msg) => {
                        if let Some(chan) = get_channel!(self.channels, channel, client) {
                            chan.msg_queue.push(Package::msg(channel, name, msg));
                            client.ack();
                        }
                    },
                    Request::Send(recp, msg) => {
                        if self.channels.get("").expect("global channel should always exist").members.contains(&recp) {
                            self.direct_msg.push((recp, Package::msg("__direct", name, msg)));
                        } else {
                            client.err("unknown user");
                        }
                    },
                    Request::Names(channel) => {
                        if let Some(chan) = get_channel!(self.channels, channel, client) {
                            client.info(chan.members.clone());
                        }
                    }
                    Request::About => client.info([&*Self::ABOUT]),
                    Request::NewChannel(channel, passwd) => {
                        if self.channels.contains_key(&channel) || &channel == "__private" {
                            client.err("channel exists already");
                        } else {
                            self.channels.insert(channel.clone(), Channel {
                                name: channel,
                                password: passwd,
                                members: vec![name.clone()],
                                msg_queue: Vec::new(),
                            });
                            client.ack();
                        }
                    },
                    Request::ListChannels => client.info(self.channels.keys()),
                    Request::Subscribe(channel, passwd) => {
                        if let Some(chan) = self.channels.get_mut(&channel) {
                            if chan.members.contains(name) {
                                client.err("already subscribed to channel");
                            } else if chan.password == passwd {
                                chan.members.push(name.clone());
                                client.ack();
                            } else {
                                client.err("wrong password");
                            }
                        } else {
                            client.err("channel doesn't exist");
                        }
                    },
                    Request::Unsubscribe(channel) => {
                        if let Some(chan) = get_channel!(self.channels, channel, client) {
                            chan.members.retain(|n| n != name);
                            client.ack();
                        }
                    },
                    Request::Unknown(cmd) => client.err(format!("unknown command {cmd}")),
                    Request::MissingArgs(args) => client.err(format!("please provide {args}")),
                }
            }
        }
    }

    fn flush_queues(&mut self) {
        for channel in self.channels.values_mut() {
            for msg in channel.msg_queue.drain(..) {
                for name in &channel.members {
                    if let Some(client) = self.active_clients.get_mut(name) {
                        client.conn.send_package(&msg);
                    }
                }
            }
        }
        for (recp, pkg) in self.direct_msg.drain(..) {
            if let Some(client) = self.active_clients.get_mut(&recp) {
                client.conn.send_package(pkg);
            }
        }
    }

    fn prune(&mut self) {
        self.active_clients.retain(|_, c| c.conn.alive());
        self.passive_clients.retain(|c| c.conn.alive());
        self.channels.retain(|_, c| {
            c.members.retain(|n| self.active_clients.contains_key(n));
            &c.name == "" || !c.members.is_empty()
        });
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
        if let Some(pkg) = conn.get_package() {
            match Request::parse(pkg) {
                Request::Login(name ) => {
                    if name.is_empty() {
                        conn.send_package(Package::err("please provide a name"));
                        continue;
                    }
                    return Some(Client {
                        conn,
                        name: Some(name),
                    });
                }
                Request::Listen => {
                    return Some(Client {
                        conn,
                        name: None,
                    });
                }
                _ => {
                    conn.send_package(Package::err("please login first"));
                }
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
    None
}

enum Request {
    Login(String),
    Listen,
    Ping,
    Post(String, String),
    Send(String, String),
    Names(String),
    About,
    NewChannel(String, String),
    ListChannels,
    Subscribe(String, String),
    Unsubscribe(String),
    Unknown(String),
    MissingArgs(&'static str),
}

impl Request {
    pub fn parse(pkg: Package) -> Self {
        match pkg.cmd.as_str() {
            "login" => {
                if pkg.args.len() >= 2 {
                    let mut args = pkg.args.into_iter();
                    Request::Login(args.next().unwrap())
                } else {
                    Request::MissingArgs("name")
                }
            }
            "listen" => Request::Listen,
            "ping" => Request::Ping,
            "post" => {
                if pkg.args.len() >= 2 {
                    let mut args = pkg.args.into_iter();
                    Request::Post(args.next().unwrap(), args.next().unwrap())
                } else {
                    Request::MissingArgs("channel, message")
                }
            }
            "send" => {
                if pkg.args.len() >= 2 {
                    let mut args = pkg.args.into_iter();
                    Request::Send(args.next().unwrap(), args.next().unwrap())
                } else {
                    Request::MissingArgs("name, message")
                }
            }
            "names" => {
                if pkg.args.len() >= 1 {
                    let mut args = pkg.args.into_iter();
                    Request::Names(args.next().unwrap())
                } else {
                    Request::MissingArgs("channel")
                }
            }
            "about" => Request::About,
            "new_channel" => {
                if pkg.args.len() >= 2 {
                    let mut args = pkg.args.into_iter();
                    Request::NewChannel(args.next().unwrap(), args.next().unwrap())
                } else {
                    Request::MissingArgs("channel, password")
                }
            }
            "list_channels" => Request::ListChannels,
            "subscribe" => {
                if pkg.args.len() >= 2 {
                    let mut args = pkg.args.into_iter();
                    Request::Subscribe(args.next().unwrap(), args.next().unwrap())
                } else {
                    Request::MissingArgs("channel, password")
                }
            }
            "unsubscribe" => {
                if pkg.args.len() >= 1 {
                    let mut args = pkg.args.into_iter();
                    Request::Unsubscribe(args.next().unwrap())
                } else {
                    Request::MissingArgs("channel")
                }
            }
            _ => Request::Unknown(pkg.cmd),
        }
    }
}
