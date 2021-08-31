#![deny(clippy::all, clippy::pedantic, rust_2018_idioms)]

use clap::Clap;
use std::{
    io::Read,
    net::{TcpStream, UdpSocket},
    str,
    str::FromStr,
};

fn main() {
    let opts = Opts::parse();

    let mut buf = [0_u8; 64];

    match opts.proto {
        Protocol::Tcp => {
            let mut client = TcpStream::connect((opts.addr, opts.port)).unwrap();
            let _ = client.read(&mut buf).unwrap();
            println!("{}", str::from_utf8(&buf).unwrap());
        }
        Protocol::Udp => {
            let addr = (opts.addr, opts.port);
            let client = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
            client.send_to(&buf, addr).unwrap();
            client.recv_from(&mut buf).unwrap();
            println!("{}", str::from_utf8(&buf).unwrap());
        }
    }
}

#[derive(Clap)]
struct Opts {
    #[clap(short, long, default_value = "tcp")]
    proto: Protocol,

    #[clap(default_value = "127.0.0.1")]
    addr: String,

    #[clap(default_value = "13")]
    port: u16,
}

enum Protocol {
    Tcp,
    Udp,
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "tcp" => Ok(Protocol::Tcp),
            "udp" => Ok(Protocol::Udp),
            _ => Err(format!("unknown protocol: {}", s)),
        }
    }
}
