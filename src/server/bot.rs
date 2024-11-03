use std::{collections::HashMap, sync::{mpsc::Receiver, Arc, Mutex}, thread};

use serde::{Deserialize, Serialize};

use crate::Package;

pub const DEFAULT_BOT_JSON: &str = include_str!("default_bot.json");

#[derive(Serialize, Deserialize)]
pub struct Bot {
    name: String,
    key: String,
    trigger: HashMap<String, Trigger>,
}

#[derive(Serialize, Deserialize)]
pub struct Trigger {
    ignore_errors: bool,
    cmds: Vec<Package>,
}

pub type BotList = Arc<Mutex<HashMap<String, Bot>>>;

pub fn load_bots() -> Vec<Bot> {
    let mut bots: Vec<Bot> = vec![serde_json::from_str(DEFAULT_BOT_JSON).unwrap()];
    if let Ok(Ok(mut user_bots)) =
        std::fs::read_to_string("bots.json").map(|s| serde_json::from_str::<Vec<Bot>>(&s))
    {
        bots.append(&mut user_bots);
    }
    bots
}

pub fn bot_thread(trigger: Receiver<String>) -> BotList {
    let mut map = HashMap::new();
    for bot in load_bots() {
        map.insert(bot.name.clone(), bot);
    }
    let ret = Arc::new(Mutex::new(map));
    let bots = ret.clone();
    thread::spawn(move || {
        while let Ok(trig) = trigger.recv() {
            for bot in bots.lock().unwrap().values() {

            }
        }
    });
    ret
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_parses() {
        let _: Bot = serde_json::from_str(DEFAULT_BOT_JSON).unwrap();
    }
}
