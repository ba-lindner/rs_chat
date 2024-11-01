use rs_chat::Server;



fn main() {
    match std::env::args().nth(1) {
        Some(s) if &s == "--server" => {
            // start server
            Server::new().run();
        }
        Some(conn) => {
            // start primary client
            let Some((_uname, _server_ip)) = conn.split_once("@") else {
                eprintln!("please provide username and server address in the format name@address");
                return;
            };
            unimplemented!("client does not exist yet");
        }
        None => {
            // start passive client
            unimplemented!("client does not exist yet");
        }
    }
}
