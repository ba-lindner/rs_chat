use std::{
    collections::HashSet,
    fmt::Display,
    io::{stdin, stdout, Write},
};

use crate::{
    connection::Connection, move_vec, package::PackageParseError, requests::Request,
    response::Response,
};

use super::{ClientErr, InterClientComm};

const START_MESSAGE: &str = concat!(
    "rs_chat secondary client v",
    env!("CARGO_PKG_VERSION"),
    "\nEnter '?' to view a list of available commands"
);

const HELP: &str = "Sending messages:
 <msg>            send <msg> to the global channel
 @<name> <msg>    send <msg> to user <name>
 /<chan> <msg>    send <msg> to channel <channel>
 use \\ to send a global message starting with a special character
Commands:
 ?                print this help
 ?<c>             print help for command <c>
 :s               get server information
 :q               quit this program
 :w [<chan>]      get name list
 :c [<arg..>]     channel operations
 :b [<name>]      block / unblock player
 :o               get your offenses
 :p <name>        pardon player";

const HELP_HELP: &str = "? - print help
Use ?<cmd> to get details about other commands";

const HELP_AT: &str = "@ - send direct messages
Usage: @<name> <message>
If no user matching the given name is found, the most similar name will be offered as an alternative. Your options are:
 [y]es: accept alternate name
 [n]o: use original name
 [a]bort: exit command";

const HELP_SLASH: &str = "/ - post messages to a channel
Usage: /<channel> <message>
Note: you must first join a channel via ':c' to post a message there";

const HELP_SERVER: &str = ":s - get server information";
const HELP_QUIT: &str = ":q - quit this program
Use :q! to also close the primary client
Note: unless you use :q!, you will stay logged in in your primary client and thus be able to read messages";

const HELP_WHO: &str = ":w - find out who is online
Usage: :w [<channel>]
Print a list of names that are subscribed to a channel. If no channel is provided, the global channel is taken.
Note: other clients may unsubscribe from the global channel and thus stay anonymous";

const HELP_CHANNEL: &str = ":c - channel operations
Several actions can be performed:
 :c - list available channels. Channels you have joined will be marked with (*).
 :c [+]<channel> [<password>] - create or join channel.\
If a channel named <channel> already exists, attempts to join that channel with the given password.\
Otherwise, the channel is created. If no password is given, an empty password is used.
 :c -[<channel>] - leave a channel. Omit the channel to leave the global channel.";

const HELP_BLOCK: &str = ":b - (un)block a user
Usage: :b [<name>]
Blocks selected user. Enter again to unblock.
If no name is given, lists the users you have currently blocked.
A name check analog to direct messages will be performed.";

const HELP_OFFENSES: &str = ":o - get your offenses
Find out how often you have offended the server and when you will be kicked";

const HELP_PARDON: &str = ":p - pardon another player
Usage: :p <name>
Reduces the number of offenses of that player by 1.
If this player did not have any offenses, it will be counted as an offense by you.
A name check analog to direct messages will be performed.";

/// Things a user might want to do
///
/// In most cases, this translates to one [`Request`].
/// However, there may be additional requests
/// prior to the main one or none at all.
enum UserCmd {
    DirectMsg(String, String),
    ChannelMsg(String, String),
    Help(Option<char>),
    ServerInfo,
    Quit(bool),
    Who(String),
    ChannelList,
    ChannelJoinNew(String, String),
    ChannelLeave(String),
    BlockList,
    Block(String),
    Offenses,
    Pardon(String),
    /// top secret, don't tell anybody
    SecretHelp,
}

/// Unexpected things that can occur
///
/// This is either an error or a signal that the user
/// decided to quit the client. An error may be significant
/// enough to cause the client to stop, but is not required to do so.
enum Happenings {
    ResponseErr(PackageParseError),
    OwnMistake(String),
    ProtocolViolation,
    ServerDied,
    QuitCmd,
}

impl From<PackageParseError> for Happenings {
    fn from(value: PackageParseError) -> Self {
        Self::ResponseErr(value)
    }
}

/// A secondary client, used to write messages and send commands
///
/// Secondary clients require a primary client
/// and can thus only be created with [`connect`](Self::connect).
/// It keeps track of its own name aswell as joined channels and blocked users
pub struct SecondaryClient {
    conn: Connection,
    name: String,
    channels: Vec<String>,
    blocked: Vec<String>,
}

impl SecondaryClient {
    /// Connect to a primary client
    ///
    /// Requires the primary client to send the metadata upon connection
    /// (see [`InterClientComm`])
    pub fn connect(port: u16) -> Result<Self, ClientErr> {
        let mut conn = Connection::to(("127.0.0.1", port))?;
        let Ok(InterClientComm::Name(name)) = conn
            .wait_package()
            .ok_or(ClientErr::StartupFailed)?
            .try_into()
        else {
            return Err(ClientErr::StartupFailed);
        };
        let Ok(InterClientComm::Channels(channels)) = conn
            .wait_package()
            .ok_or(ClientErr::StartupFailed)?
            .try_into()
        else {
            return Err(ClientErr::StartupFailed);
        };
        let Ok(InterClientComm::Blocked(blocked)) = conn
            .wait_package()
            .ok_or(ClientErr::StartupFailed)?
            .try_into()
        else {
            return Err(ClientErr::StartupFailed);
        };
        Ok(Self {
            conn,
            name,
            channels,
            blocked,
        })
    }

    /// run the client
    /// 
    /// this function will only return when either an unrecoverable error has occured
    /// or the user decided to quit (see [`Happenings`])
    pub fn run(&mut self) {
        println!("{}", START_MESSAGE);
        loop {
            let inp = get_line("> ");
            if let Some(cmd) = Self::parse_input(inp.trim()) {
                if let Err(why) = self.exec_cmd(cmd) {
                    match why {
                        Happenings::ResponseErr(err) => {
                            eprintln!("server sent invalid response: {err}")
                        }
                        Happenings::ProtocolViolation => eprintln!("server violated the protocol"),
                        Happenings::ServerDied => eprintln!("server died. oh no."),
                        Happenings::QuitCmd => {}
                        Happenings::OwnMistake(what) => {
                            eprintln!("you made a mistake (or me?): {what}");
                            continue;
                        }
                    }
                    return;
                }
            }
        }
    }

    /// try to figure out what the user intended to do
    fn parse_input(inp: &str) -> Option<UserCmd> {
        let trimmed = inp.get(1..)?.trim_start();
        Some(match inp.chars().next()? {
            '@' => {
                let Some((name, msg)) = trimmed.split_once(char::is_whitespace) else {
                    eprintln!("please provide name and message");
                    return None;
                };
                UserCmd::DirectMsg(name.to_string(), msg.to_string())
            }
            '/' => {
                let Some((channel, msg)) = trimmed.split_once(char::is_whitespace) else {
                    eprintln!("please provide channel and message");
                    return None;
                };
                UserCmd::ChannelMsg(channel.to_string(), msg.to_string())
            }
            '\\' => UserCmd::ChannelMsg(String::new(), trimmed.to_string()),
            ':' => Self::parse_cmd(trimmed)?,
            '?' => UserCmd::Help(trimmed.chars().next()),
            _ => UserCmd::ChannelMsg(String::new(), inp.to_string()),
        })
    }

    fn parse_cmd(inp: &str) -> Option<UserCmd> {
        let (cmd, args) = if let Some((cmd, args)) = inp.split_once(char::is_whitespace) {
            (cmd, args.split_whitespace().collect())
        } else {
            (inp, Vec::new())
        };
        Some(match cmd {
            "s" => UserCmd::ServerInfo,
            "q" => UserCmd::Quit(false),
            "q!" => UserCmd::Quit(true),
            "w" => UserCmd::Who(args.first().map(|a| a.to_string()).unwrap_or_default()),
            "c" => {
                if let Some(channel) = args.first() {
                    if let Some(channel) = channel.strip_prefix('-') {
                        UserCmd::ChannelLeave(channel.to_string())
                    } else {
                        let chan = channel.strip_prefix('+').unwrap_or(channel).to_string();
                        UserCmd::ChannelJoinNew(
                            chan,
                            args.get(1).map(|a| a.to_string()).unwrap_or_default(),
                        )
                    }
                } else {
                    UserCmd::ChannelList
                }
            }
            "b" => {
                if let Some(name) = args.first() {
                    UserCmd::Block(name.to_string())
                } else {
                    UserCmd::BlockList
                }
            }
            "o" => UserCmd::Offenses,
            "p" => {
                if let Some(arg) = args.first() {
                    UserCmd::Pardon(arg.to_string())
                } else {
                    eprintln!("please provide a name");
                    return None;
                }
            }
            "?!" => UserCmd::SecretHelp,
            _ => {
                eprintln!("unknown command {cmd}");
                return None;
            }
        })
    }

    fn exec_cmd(&mut self, cmd: UserCmd) -> Result<(), Happenings> {
        match cmd {
            UserCmd::Help(sub) => {
                let help_txt = match sub {
                    None => HELP,
                    Some('?') => HELP_HELP,
                    Some('@') => HELP_AT,
                    Some('/') => HELP_SLASH,
                    Some('s') => HELP_SERVER,
                    Some('q') => HELP_QUIT,
                    Some('w') => HELP_WHO,
                    Some('c') => HELP_CHANNEL,
                    Some('b') => HELP_BLOCK,
                    Some('o') => HELP_OFFENSES,
                    Some('p') => HELP_PARDON,
                    Some(c) => {
                        eprintln!("unknown command {c}");
                        return Ok(());
                    }
                };
                println!("{help_txt}");
            }
            UserCmd::SecretHelp => {
                println!("visit https://github.com/ba-lindner/rs_chat for more help")
            }
            UserCmd::BlockList => println!("you have currently blocked: {}", Disp(&self.blocked)),
            UserCmd::Quit(force) => {
                if force {
                    self.conn.send_package(InterClientComm::Quit.package());
                }
                return Err(Happenings::QuitCmd);
            }
            UserCmd::ChannelList => println!(
                "channels: {}",
                Disp(
                    &self
                        .info_request(Request::ListChannels)?
                        .into_iter()
                        .map(|c| {
                            if self.channels.contains(&c) {
                                format!("(*) {}", channel_name(&c))
                            } else {
                                channel_name(&c).to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                )
            ),
            UserCmd::ServerInfo => {
                println!(
                    "server: {}",
                    self.info_request(Request::About)?
                        .first()
                        .ok_or(Happenings::ProtocolViolation)?
                );
                println!(
                    "available features: {}",
                    Disp(&self.info_request(Request::Features)?)
                );
            }
            UserCmd::Offenses => {
                let [own, max] = move_vec(self.info_request(Request::Offenses)?)
                    .ok_or(Happenings::ProtocolViolation)?;
                println!("your offenses: {own} / {max}");
            }
            UserCmd::Who(chan) => {
                if self.channels.contains(&chan) {
                    println!(
                        "members of channel {}: {}",
                        channel_name(&chan),
                        Disp(&self.info_request(Request::Names(chan.clone()))?)
                    );
                } else {
                    eprintln!("join channel {} to list its members", channel_name(&chan));
                }
            }
            UserCmd::ChannelMsg(chan, msg) => {
                if self.channels.contains(&chan) {
                    self.ack_request(Request::Post(chan, msg))?;
                } else {
                    eprintln!("join channel {} to post messages", channel_name(&chan))
                }
            }
            UserCmd::DirectMsg(name, msg) => {
                if let Some(name) = self.check_user(name)? {
                    if self.blocked.contains(&name) {
                        eprintln!("you blocked {name}");
                    } else {
                        self.ack_request(Request::Send(name, msg))?;
                    }
                }
            }
            UserCmd::Block(name) => {
                if let Some(name) = self.check_user(name)? {
                    if name == self.name {
                        eprintln!("don't block yourself :(");
                    } else if self.blocked.contains(&name) {
                        self.ack_request(Request::Unblock(name.clone()))?;
                        self.blocked.retain(|n| *n != name);
                        println!("unblocked {name}");
                    } else {
                        self.ack_request(Request::Block(name.clone()))?;
                        println!("blocked {name}");
                        self.blocked.push(name);
                    }
                    self.conn
                        .send_package(InterClientComm::Blocked(self.blocked.clone()).package());
                }
            }
            UserCmd::Pardon(name) => {
                if name == self.name {
                    eprintln!("can't pardon yourself");
                } else if let Some(name) = self.check_user(name)? {
                    self.ack_request(Request::Pardon(name.clone()))?;
                    println!("pardoned {name}")
                }
            }
            UserCmd::ChannelLeave(channel) => {
                if self.channels.contains(&channel) {
                    self.ack_request(Request::Unsubscribe(channel.clone()))?;
                    self.channels.retain(|c| *c != channel);
                    println!("left {}", channel_name(&channel));
                    self.conn
                        .send_package(InterClientComm::Channels(self.channels.clone()).package());
                } else {
                    eprintln!("you didn't join {}", channel_name(&channel));
                }
            }
            UserCmd::ChannelJoinNew(channel, passwd) => {
                if self.channels.contains(&channel) {
                    eprintln!("you are already in {}", channel_name(&channel))
                }
                if self.info_request(Request::ListChannels)?.contains(&channel) {
                    self.ack_request(Request::Subscribe(channel.clone(), passwd))?;
                    println!("joined {}", channel_name(&channel));
                } else {
                    self.ack_request(Request::NewChannel(channel.clone(), passwd))?;
                    println!("created {}", channel_name(&channel));
                }
                self.channels.push(channel);
                self.conn
                    .send_package(InterClientComm::Channels(self.channels.clone()).package());
            }
        }
        Ok(())
    }

    fn check_user(&mut self, name: String) -> Result<Option<String>, Happenings> {
        let mut all_names = HashSet::new();
        for user in self.info_request(Request::Names(String::new()))? {
            all_names.insert(user);
        }
        if all_names.contains(&name) {
            return Ok(Some(name));
        }
        let channels = self.channels.clone();
        for chan in channels {
            if chan.is_empty() {
                continue;
            }
            for user in self.info_request(Request::Names(chan.clone()))? {
                all_names.insert(user);
            }
        }
        if all_names.contains(&name) {
            return Ok(Some(name));
        }
        let Some((best, _)) = all_names
            .into_iter()
            .map(|n| {
                let diff = strsim::jaro_winkler(&name, &n);
                (n, diff)
            })
            .max_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
        else {
            eprintln!("no names found");
            return Ok(None);
        };
        let answer = get_line(&format!(
            "user {name} not found. did you mean {best}? [y/n/a] "
        ));
        Ok(match answer.trim().chars().next() {
            Some('y') => Some(best),
            Some('n') => Some(name),
            _ => None,
        })
    }

    fn info_request(&mut self, req: Request) -> Result<Vec<String>, Happenings> {
        self.conn.send_package(req.package());
        let pkg = self.conn.wait_package().ok_or(Happenings::ServerDied)?;
        match pkg.try_into()? {
            Response::Info(data) => Ok(data),
            Response::Err(why) => Err(Happenings::OwnMistake(why)),
            _ => Err(Happenings::ProtocolViolation),
        }
    }

    fn ack_request(&mut self, req: Request) -> Result<(), Happenings> {
        self.conn.send_package(req.package());
        let pkg = self.conn.wait_package().ok_or(Happenings::ServerDied)?;
        match pkg.try_into()? {
            Response::Ack => Ok(()),
            Response::Err(why) => Err(Happenings::OwnMistake(why)),
            _ => Err(Happenings::ProtocolViolation),
        }
    }
}

struct Disp<'d>(&'d [String]);

impl<'d> Display for Disp<'d> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, arg) in self.0.iter().enumerate() {
            if idx == 0 {
                write!(f, "{}", arg)?;
            } else {
                write!(f, ", {}", arg)?;
            }
        }
        Ok(())
    }
}

fn get_line(prompt: &str) -> String {
    print!("{}", prompt);
    stdout()
        .flush()
        .expect("stdout should be available for cli");
    let mut answer = String::new();
    stdin()
        .read_line(&mut answer)
        .expect("stdin should be available for cli");
    answer
}

fn channel_name(chan: &str) -> &str {
    if chan.is_empty() {
        "<GLOBAL>"
    } else {
        chan
    }
}
