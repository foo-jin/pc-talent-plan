use building_blocks::{Ping, PingResponse};
use std::{io::BufReader, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6380").unwrap();
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let req: Ping = building_blocks::from_reader(BufReader::new(&mut stream)).unwrap();
        let rsp = match req.msg {
            None => PingResponse::Pong,
            Some(msg) => PingResponse::Echo(msg),
        };
        building_blocks::to_writer(&mut stream, &rsp).unwrap();
    }
}
