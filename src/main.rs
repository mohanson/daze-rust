use std::io::{Read, Write};

fn salt(s: &str) -> Vec<u8> {
    let mut cipher: Vec<u8> = md5::compute(s).to_vec();
    cipher.extend(cipher.clone());
    cipher.extend(cipher.clone());
    cipher.extend(cipher.clone());
    cipher
}

fn daze(src_stream: &std::net::TcpStream, k: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let mut src_reader = src_stream.try_clone()?;
    let src_writer = src_stream.try_clone()?;
    let mut key: Vec<u8> = vec![0; 256];
    src_reader.read_exact(&mut key[0..128])?;
    key[128..256].copy_from_slice(k);
    let mut src_reader = rc4::Reader::new(src_reader, key.as_slice())?;
    let mut src_writer = rc4::Writer::new(src_writer, key.as_slice())?;

    let mut buf: Vec<u8> = vec![0; 10];
    src_reader.read_exact(&mut buf)?;
    let pit = u64::from_be_bytes(buf[0..8].try_into().unwrap());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let sub = if now > pit { now - pit } else { pit - now };
    if sub > 120 {
        return Err("daze: request expired".into());
    };
    let mut buf: Vec<u8> = vec![0; buf[9] as usize];
    src_reader.read_exact(&mut buf)?;
    let dst = String::from_utf8(buf).unwrap();
    println!("conn:   dial network=tcp address={}", dst);

    let dst_stream = std::net::TcpStream::connect(&dst)?;
    let mut dst_reader = dst_stream.try_clone()?;
    let mut dst_writer = dst_stream.try_clone()?;
    src_writer.write_all(&[0])?;
    std::thread::spawn(move || {
        std::io::copy(&mut src_reader, &mut dst_writer).ok();
        dst_stream.shutdown(std::net::Shutdown::Both).unwrap();
    });
    std::io::copy(&mut dst_reader, &mut src_writer).ok();
    src_stream.shutdown(std::net::Shutdown::Both).unwrap();
    Ok(())
}

fn hand(src_stream: &std::net::TcpStream, k: &[u8]) {
    if let Err(err) = daze(src_stream, k) {
        println!("conn:  error {}", err);
    }
    println!("conn: closed")
}

fn main() {
    let mut c_listen = String::new();
    let mut c_cipher = String::from("daze");

    {
        let mut ap = argparse::ArgumentParser::new();
        ap.refer(&mut c_listen)
            .add_option(&["-l"], argparse::Store, "Listen address");
        ap.refer(&mut c_cipher)
            .add_option(&["-k"], argparse::Store, "Password");
        ap.parse_args_or_exit();
    }

    println!("main: server cipher is {}", c_cipher);
    let cipher = salt(&c_cipher);
    println!("main: listen and server on {}", c_listen);
    let listener = std::net::TcpListener::bind(&c_listen[..]).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("conn: accept remote={}", stream.peer_addr().unwrap());
        let cipher = cipher.clone();
        std::thread::spawn(move || hand(&stream, cipher.as_slice()));
    }
}
