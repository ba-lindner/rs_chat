use std::{
    io::stdin,
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use crate::connect::{Connection, Package};

use super::{server_connection, ClientErr};

pub struct TrivialClient {
    conn: Connection,
}

impl TrivialClient {
    pub fn connect(addr: &str, name: &str) -> Result<Self, ClientErr> {
        Ok(Self {
            conn: server_connection(addr, name)?,
        })
    }

    pub fn run(&mut self) {
        println!("rs_chat trivial client v{}", env!("CARGO_PKG_VERSION"));
        let (tx, rx) = channel();
        input_thread(tx);
        loop {
            if let Ok(pkg) = rx.try_recv() {
                self.conn.send_package(pkg);
            }
            self.conn.get_package();
            if !self.conn.alive() {
                println!("connection was lost");
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

fn input_thread(tx: Sender<Package>) {
    thread::spawn(move || loop {
        let mut inp = String::new();
        stdin().read_line(&mut inp).unwrap();
        tx.send(inp_to_package(inp.trim_end())).unwrap()
    });
}

fn inp_to_package(inp: &str) -> Package {
    let Some((cmd, args)) = inp.split_once(' ') else {
        return Package {
            cmd: inp.to_string(),
            args: Vec::new(),
        };
    };
    Package {
        cmd: cmd.to_string(),
        args: lex_args(args),
    }
}

fn lex_args(args: &str) -> Vec<String> {
    let mut chars = args.chars();
    let mut res = Vec::new();
    let mut curr_word = String::new();
    let (mut quoted, mut was_quoted) = (false, false);
    while let Some(c) = chars.next() {
        if curr_word.is_empty() && !quoted && c == '"' {
            quoted = true;
        } else if quoted && c == '"' {
            quoted = false;
            was_quoted = true;
        } else if !quoted && c == ' ' {
            res.push(curr_word);
            curr_word = String::new();
            was_quoted = false;
        } else if quoted && c == '\\' {
            match chars.next() {
                Some('"') => curr_word += "\"",
                Some('\\') => curr_word += "\\",
                Some(ch) => {
                    curr_word.push('\\');
                    curr_word.push(ch);
                }
                None => curr_word += "\\",
            }
        } else {
            curr_word.push(c);
        }
    }
    if !curr_word.is_empty() || was_quoted {
        res.push(curr_word);
    }
    res
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_arg_lexer(args: &str, expected: &[&str]) {
        let res = lex_args(args);
        assert_eq!(&res, expected);
    }

    #[test]
    fn arg_lexing() {
        test_arg_lexer("args", &["args"]);
        test_arg_lexer("a b c", &["a", "b", "c"]);
        test_arg_lexer(r#""a" "b""#, &["a", "b"]);
        test_arg_lexer(r#""a \" b""#, &["a \" b"]);
        test_arg_lexer(r#"a "b c" \a"#, &["a", "b c", "\\a"]);
        test_arg_lexer("a\"b c\"d\"", &["a\"b", "c\"d\""]);
        test_arg_lexer("\" \"args \"a  bc", &[" args", "a  bc"]);
        test_arg_lexer("\"\" \"a b c", &["", "a b c"]);
        test_arg_lexer("\"\"", &[""]);
    }
}
