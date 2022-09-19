use std::{net::UdpSocket, borrow::Cow};

fn main() {
    let socket = UdpSocket::bind("127.0.0.1:8081").expect("couldn't bind to address");
    loop {
        let mut buf = [0; 100];
        let (amt, src) = socket.recv_from(&mut buf).map_err(|e| e.to_string()).unwrap();
        let buf = &mut buf[..amt];
       // let result = Vec::from(&buf[0..amt]);
        let invalidBuf = buf.iter().any(|&x| {println!("{}", x); x > 127} );
        if (invalidBuf) {
            println!("invalid data");
            continue;
        }
        let data  = String::from_utf8(buf.to_vec()).unwrap();
        println!("received data - {}", data);
    }

}