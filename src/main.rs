extern crate argparse;
extern crate byteorder;
extern crate md5;
extern crate rc4;

use byteorder::ByteOrder;
use std::error;
use std::io;
use std::io::prelude::*;
use std::net;
use std::thread;
use std::time;

fn daze(src_stream: &net::TcpStream, k: &[u8]) -> Result<(), Box<error::Error>> {
    let mut src_reader = src_stream.try_clone()?;
    let mut src_writer = src_stream.try_clone()?;
    let mut key: Vec<u8> = vec![0; 128];
    src_reader.read_exact(&mut key)?;
    key.append(&mut Vec::from(k));
    let mut src_reader = rc4::Reader::init(src_reader, key.as_slice())?;

    let mut buf: Vec<u8> = vec![0; 12];
    src_reader.read_exact(&mut buf)?;
    if buf[0] != 0xFF || buf[1] != 0xFF {
        return Err(From::from(format!(
            "daze: malformed request: {:?}",
            &buf[..2]
        )));
    };
    let pit = byteorder::BigEndian::read_u64(&buf[2..10]);
    let now = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)?
        .as_secs();
    let sub = if now > pit { now - pit } else { pit - now };
    if sub > 120 {
        return Err(From::from(format!("daze: time span is too large: {}", sub)));
    };
    let mut buf: Vec<u8> = vec![0; buf[11] as usize];
    src_reader.read_exact(&mut buf)?;
    let dst = String::from_utf8(buf).unwrap();
    println!("Connect {}", dst);

    let dst_stream = net::TcpStream::connect(&dst)?;
    let dst_reader = dst_stream.try_clone()?;
    let mut dst_reader = rc4::Reader::init(dst_reader, key.as_slice())?;
    let mut dst_writer = dst_stream.try_clone()?;

    thread::spawn(move || {
        io::copy(&mut src_reader, &mut dst_writer).ok();
    });
    io::copy(&mut dst_reader, &mut src_writer).ok();
    Ok(())
}

fn hand(src_stream: &net::TcpStream, k: &[u8]) {
    if let Err(err) = daze(src_stream, k) {
        println!("{:?}", err);
    }
}

fn main() {
    let mut c_listen = String::from("127.0.0.1:51958");
    let mut c_cipher = String::from("daze");

    {
        let mut ap = argparse::ArgumentParser::new();
        ap.set_description("Start daze server");
        ap.refer(&mut c_listen)
            .add_option(&["-l"], argparse::Store, "listen address");
        ap.refer(&mut c_cipher)
            .add_option(&["-k"], argparse::Store, "cipher, for encryption");
        ap.parse_args_or_exit();
    }

    let cipher: Vec<u8> = md5::compute(c_cipher).0.to_vec();
    println!("Listen and server on {}", c_listen);
    let listener = net::TcpListener::bind(&c_listen[..]).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let cipher = cipher.clone();
        thread::spawn(move || hand(&stream, cipher.as_slice()));
    }
}
