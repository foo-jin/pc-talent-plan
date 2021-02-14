use building_blocks::{Ping, PingResponse};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:6380").unwrap();
    let out = io::stdout();

    for l in io::stdin().lock().lines().map(Result::unwrap) {
        let mut w = out.lock();
        let parts: Vec<&str> = l.splitn(2, char::is_whitespace).collect();

        let req = match parts[..] {
            ["PING"] => Ping::empty(),
            ["PING", rest] => Ping::with_msg(rest),
            _ => continue,
        };
        building_blocks::to_writer(&mut stream, &req).unwrap();
        let rsp: PingResponse = building_blocks::from_reader(BufReader::new(&mut stream)).unwrap();
        let rsp_content = match rsp {
            PingResponse::Pong => "PONG".to_owned(),
            PingResponse::Echo(s) => s,
        };
        writeln!(&mut w, "{}", rsp_content).unwrap();
    }
}
