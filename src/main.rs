extern crate argparse;
extern crate byteorder;
extern crate crypto;
extern crate md5;

use byteorder::ByteOrder;
use crypto::rc4::Rc4;
use crypto::symmetriccipher::SynchronousStreamCipher;
use std::error;
use std::io::prelude::*;
use std::net;
use std::sync;
use std::thread;
use std::time;

fn read(
    stream: &mut net::TcpStream,
    c: &mut Rc4,
    buf: &mut [u8],
) -> Result<usize, Box<error::Error>> {
    let mut src = vec![0; buf.len()];
    let n = stream.read(&mut src)?;
    c.process(&src[..n], &mut buf[..n]);
    Ok(n)
}

fn daze(mut src_stream: net::TcpStream, k: &str) {
    let mut buf: Vec<u8> = vec![0; 128];
    if src_stream.read(&mut buf).is_err() {
        return;
    }
    let mut raw: Vec<u8> = md5::compute(k).0.iter().cloned().collect();
    buf.append(&mut raw);
    let mut cipher_a = Rc4::new(&buf);
    let mut cipher_b = Rc4::new(&buf);
    let mut buf: Vec<u8> = vec![0; 12];
    if read(&mut src_stream, &mut cipher_a, &mut buf).is_err() {
        return;
    };
    if buf[0] != 0xFF || buf[1] != 0xFF {
        println!("daze: malformed request: {:?}", &buf[..2]);
        return;
    };
    let pit = byteorder::BigEndian::read_u64(&buf[2..10]);
    let now = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let c = if now > pit { now - pit } else { pit - now };
    if c > 120 {
        println!("daze: time span is too large: {}", c);
        return;
    };
    let mut buf: Vec<u8> = vec![0; buf[11] as usize];
    if read(&mut src_stream, &mut cipher_a, &mut buf).is_err() {
        return;
    };
    let dst = String::from_utf8(buf).unwrap();
    println!("Connect {}", dst);

    let mut dst_stream = net::TcpStream::connect(&dst).unwrap();
    let mut src_stream_copy = src_stream.try_clone().unwrap();
    let mut dst_stream_copy = dst_stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut buf: Vec<u8> = vec![0; 32 * 1024];
        loop {
            let n = match read(&mut src_stream_copy, &mut cipher_a, &mut buf) {
                Ok(data) => data,
                _ => 0,
            };
            if n == 0 {
                break;
            };
            if dst_stream_copy.write_all(&buf[..n]).is_err() {
                break;
            };
        }
        src_stream_copy.shutdown(net::Shutdown::Both).unwrap();
        dst_stream_copy.shutdown(net::Shutdown::Both).unwrap();
    });

    let mut buf: Vec<u8> = vec![0; 32 * 1024];
    loop {
        let n = match read(&mut dst_stream, &mut cipher_b, &mut buf) {
            Ok(data) => data,
            _ => 0,
        };
        if n == 0 {
            break;
        };
        if src_stream.write_all(&buf[..n]).is_err() {
            break;
        };
    }
    dst_stream.shutdown(net::Shutdown::Both).unwrap();
    src_stream.shutdown(net::Shutdown::Both).unwrap();
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

    println!("Listen and server on {}", c_listen);
    let listener = net::TcpListener::bind(&c_listen[..]).unwrap();
    let c_cipher = sync::Arc::new(c_cipher);
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let c_cipher = c_cipher.clone();
        thread::spawn(move || daze(stream, &c_cipher[..]));
    }
}
