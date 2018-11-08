extern crate crypto;
extern crate md5;

use crypto::rc4::Rc4;
use std::error::Error;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time;

use crypto::symmetriccipher::SynchronousStreamCipher;

static c_listen: &str = "0.0.0.0:51958";
static c_cipher: &str = "daze";

fn read_exact(
    mut stream: &mut TcpStream,
    c: &mut Rc4,
    mut buf: &mut [u8],
) -> Result<(), Box<Error>> {
    let mut src = vec![0; buf.len()];
    stream.read_exact(&mut src)?;
    c.process(&src, buf);
    Ok(())
}

fn read(mut stream: &mut TcpStream, c: &mut Rc4, mut buf: &mut [u8]) -> Result<usize, Box<Error>> {
    let mut src = vec![0; buf.len()];
    let n = stream.read(&mut src)?;
    c.process(&src, buf);
    Ok(n)
}

fn daze(mut stream: TcpStream) {
    let mut buf: Vec<u8> = vec![0; 128];
    stream.read_exact(&mut buf).unwrap();
    let mut raw: Vec<u8> = md5::compute(c_cipher).0.iter().cloned().collect();
    buf.append(&mut raw);
    let mut c = Rc4::new(&buf);
    let mut z = Rc4::new(&buf);
    let mut buf: Vec<u8> = vec![0; 12];
    if let Err(err) = read_exact(&mut stream, &mut c, &mut buf) {
        println!("{}", err);
        return;
    };
    if buf[0] != 0xFF || buf[1] != 0xFF {
        println!("daze: malformed request: {:?}", &buf[..2]);
        return;
    }
    let mut buf: Vec<u8> = vec![0; buf[11] as usize];
    if let Err(err) = read_exact(&mut stream, &mut c, &mut buf) {
        println!("{}", err);
        return;
    };
    let dst = String::from_utf8(buf).unwrap();
    println!("Connect {}", dst);

    let mut dst_stream = TcpStream::connect(&dst).unwrap();
    let mut t_stream = stream.try_clone().unwrap();
    let mut t_dst_stream = dst_stream.try_clone().unwrap();

    thread::spawn(move || loop {
        let mut buf: Vec<u8> = vec![0; 32*1024];
        match read(&mut t_stream, &mut c, &mut buf) {
            Ok(data) => {
                if let Err(err) = &t_dst_stream.write(&buf[..data]) {
                    break;
                }
                continue;
            }
            Err(err) => break,
        }
        thread::sleep(time::Duration::from_secs(1));
    });

    let mut buf: Vec<u8> = vec![0; 32*1024];
    loop {
        match read(&mut dst_stream, &mut z, &mut buf) {
            Ok(data) => {
                if let Err(err) = stream.write(&buf[..data]) {
                    break;
                }
                continue;
            }
            Err(err) => break,
        }
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn main() {
    let listener = TcpListener::bind(c_listen).unwrap();
    println!("Listen and server on {}", c_listen);
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        thread::spawn(move || daze(stream));
    }
}
