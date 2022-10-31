use tokio::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddr, IpAddr};
use log;
mod cli;
use env_logger;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio_tun::TunBuilder;

async fn conn (remote_addr: String, port: u16) {
    log::info!("Starting as client");
    let tun = TunBuilder::new()
    .name("")
    .address(Ipv4Addr::new(10,0,0,5))
    .netmask(Ipv4Addr::new(255,255,255,0))         // if name is empty, then it is set by kernel.
    .tap(false)   // false (default): TUN, true: TAP.
    .packet_info(false)  // false: IFF_NO_PI, default is true.
    .up()                // or set it up manually using `sudo ip link set <tun-name> up`.
    .try_build();

    let tun = match tun {
        Ok(tun) => tun,
        Err(_err) => { panic!("yo") }
    };
    let (mut reader, mut writer) = tokio::io::split(tun);
    let socket = UdpSocket::bind("0.0.0.0:8080").await;
    let socket = match socket {
        Ok(socket) => socket,
        Err(_err) => { panic!("yo") }
    };
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let raddr = remote_addr + ":" + &port.to_string();
    let mut sock_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),8080);
    let mut len = 0;
    let mut len1 = 0;

    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from server");
                (len, sock_addr) = socket.recv_from(&mut buf2).await.expect("error receving from socket");

                log::debug!("origin {}, server -{:?}", sock_addr, &buf2[0..len]);
            } => {
                    log::debug!("writting to interface");
                    writer.write(&mut buf2[0..len]).await.expect("error while writting to interface");
            },
            _ = async {
                log::debug!("reading from interface");
                len1 = reader.read(&mut buf1).await.expect("sas");
                log::debug!("interface -{:?}", &buf1[0..len1]);
            } => {
                    log::debug!("sending to server {}", &raddr);
                    socket.send_to(&buf1[0..len1], &raddr).await.expect("error whle sending to socket");
            } 
        };
    }
}

async fn serv (port: &u16)  {
    log::info!("Starting as server");
    let tun = TunBuilder::new()
    .name("")
    .address(Ipv4Addr::new(10,0,0,6))
    .netmask(Ipv4Addr::new(255,255,255,0))         // if name is empty, then it is set by kernel.
    .tap(false)   // false (default): TUN, true: TAP.
    .packet_info(false)  // false: IFF_NO_PI, default is true.
    .up()                // or set it up manually using `sudo ip link set <tun-name> up`.
    .try_build();

    let tun = match tun {
        Ok(tun) => tun,
        Err(_err) => { panic!("yo") }
    };

    let (mut reader, mut writer) = tokio::io::split(tun);

    let socket = UdpSocket::bind(String::from("0.0.0.0:") + &port.to_string()).await;
    let socket = match socket {
        Ok(socket) => socket,
        Err(_err) => { panic!("yo") }
    };
    let mut buf2 = [0u8; 1500];
    let mut buf1 = [0u8; 1500];
    let mut caddr: String = "null".to_string();
    let mut len = 0;
    let mut len1 = 0;
    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from interface");
                len = reader.read(&mut buf1).await.expect("error while reading from interface");
                log::debug!("interface - {:?}", &buf1[0..len]);
            } => {
                    log::debug!("sending to client without addr");
                    if caddr != String::from("null") {
                        log::debug!("sending to client {}", &caddr);
                        socket.send_to(&buf1[0..len], &caddr).await.expect("error in writting to socket"); 
                    }        
                },
            _ = async {
                log::debug!("reading from client");
                let tup = socket.recv_from(&mut buf2).await;
                match tup {
                    Ok((_amt, addr)) => {
                        caddr = addr.to_string();
                        len1 = _amt;
                    }
                    Err(err) => {
                        log::error!("reading from client failed with error {}", err.to_string());
                    }
                }
                log::debug!("client - {:?}", &buf2[0..len1]);
            } => {
                    log::debug!("writting to interface");
                    writer.write(&mut buf2[0..len1]).await.expect("error while writting to interfae");
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