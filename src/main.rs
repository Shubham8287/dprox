use std::io::{Read, Write};
use std::net::UdpSocket;
use tun::platform::Device;
use log;
mod cli;
use env_logger;

extern crate tun;

fn create_tun(addr: String) -> Device {
	let mut config = tun::Configuration::default();
	config.address(addr)
	       .netmask((255, 255, 255, 0))
	       .up();

	#[cfg(target_os = "linux")]
	config.platform(|config| {
		config.packet_information(true);
	});

	tun::create(&config).unwrap()
}


async fn conn (remote_addr: String, port: u16) {
    log::info!("Starting as client");
    let mut tun = create_tun("10.0.0.5".to_string());
    let socket = UdpSocket::bind(String::from("127.0.0.1:8080")).unwrap();
    let mut buf1 = [0u8; 2048];
    let mut buf2 = [0u8; 2048];
    let raddr = remote_addr + ":" + &port.to_string();
    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from server");
                socket.recv_from(&mut buf2).unwrap();
            } => {
                    log::debug!("writting to interface");
                    tun.write(&mut buf2).unwrap()
            },
            _ = async {
                log::debug!("reading from interface");
                tun.read(&mut buf1).unwrap();
            } => {
                    log::debug!("sending to server {}", &raddr);
                    socket.send_to(&buf1, &raddr).unwrap()
            } 
        };
    }
}

async fn serv (port: &u16) {
    log::info!("Starting as server");
    let mut tun = create_tun("10.1.0.6".to_string());
    let socket = UdpSocket::bind(String::from("127.0.0.1:") + &port.to_string()).unwrap();
    let mut buf1 = [0u8; 2048];
    let mut buf2 = [0u8; 2048];
    let mut caddr: String = "null".to_string();
    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from interface");
                tun.read(&mut buf1).unwrap();
            } => {
                    log::debug!("sending to client without addr");
                    if caddr != String::from("null") {
                        log::debug!("sending to client {}", &caddr);
                        socket.send_to(&buf1, &caddr).unwrap(); 
                    }        
                },
            _ = async {
                log::debug!("reading from client");
                let (_amt, addr) = socket.recv_from(&mut buf2).unwrap();
                caddr = addr.to_string();
            } => {
                    log::debug!("writting to interface");
                    tun.write(&mut buf2).unwrap();
            }
        };
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    match cli::get_args().unwrap() {
        cli::Args::Client(client) => conn(client.remote_addr, client.port).await,
        cli::Args::Server(server) => serv(&server.port).await,
    }
    // let mut dev = create_tun("10.0.0.2".to_string());
    // let mut dev1 = create_tun("10.1.0.2".to_string());
	// let mut buf = [0; 1024];

    // let socket = UdpSocket::bind("localhost:8080").expect("couldn't bind to address");

    // let socket1 = UdpSocket::bind("127.0.0.1:8081").expect("couldn't bind to address");

	// loop {
	// 	let amount = dev.read(&mut buf).unwrap();
	// 	println!("received from tap is - {:?}", &buf[0 .. amount]);
    //     socket.send_to(&buf, "127.0.0.1:8081");

    //     socket1.recv_from(&mut buf).unwrap();
    //     print!("at server {:?}", &buf);
    //     dev1.write(&mut buf).unwrap();
    //     let amoun = dev1.read(&mut buf ).unwrap();
    //     socket1.send_to(&buf, "127.0.0.1:8080");
    //     socket.recv_from(&mut buf).unwrap();
    //     dev.write(&mut buf).unwrap();

	// }
}