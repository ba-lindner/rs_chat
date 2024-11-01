use rs_chat::Server;



fn main() {
    match std::env::args().nth(1) {
        Some(s) if &s == "--server" => {
            // start server
            Server::new().run();
        }
        Some(conn) => {
            // start primary client
            let Some((uname, server_ip)) = conn.split_once("@") else {
                eprintln!("please provide username and server address in the format name@address");
                return;
            };
        }
        None => {
            // start passive client
        }
    }
}
