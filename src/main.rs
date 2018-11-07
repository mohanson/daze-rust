extern crate crypto;
extern crate md5;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;

static c_listen: &str = "0.0.0.0:51958";
static c_cipher: &str = "daze";

fn daze(mut stream: TcpStream) {
    let mut buf: Vec<u8> = vec![0; 128];
    stream.read_exact(&mut buf).unwrap();
    let mut raw: Vec<u8> = md5::compute(c_cipher).0.iter().cloned().collect();
    println!("{:?}", raw);
    buf.append(&mut raw);
}

fn main() {
    let listener = TcpListener::bind(c_listen).unwrap();
    println!("Listen and server on {}", c_listen);
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        thread::spawn(move || daze(stream));
    }
}
