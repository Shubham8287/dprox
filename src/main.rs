use tokio::net::UdpSocket;
use std::net::Ipv4Addr;
use log;
mod cli;
use env_logger;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio_tun::TunBuilder;

async fn conn (remote_addr: String, port: u16) {
    log::info!("Starting as client");
    let tun = TunBuilder::new()
    .name("")
    .address(Ipv4Addr::new(10,0,0,5))         // if name is empty, then it is set by kernel.
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
    let mut buf1 = [0u8; 512];
    let mut buf2 = [0u8; 512];
    let raddr = remote_addr + ":" + &port.to_string();
    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from server");
                socket.recv_from(&mut buf2).await;
                log::debug!("server -{:?}", buf2);
            } => {
                    log::debug!("writting to interface");
                    writer.write(&mut buf2).await;
            },
            _ = async {
                log::debug!("reading from interface");
                reader.read(&mut buf1).await;
                log::debug!("interface -{:?}", buf1);
            } => {
                    log::debug!("sending to server {}", &raddr);
                    socket.send_to(&buf1, &raddr).await;
            } 
        };
    }
}

async fn serv (port: &u16)  {
    log::info!("Starting as server");
    let tun = TunBuilder::new()
    .name("")
    .address(Ipv4Addr::new(10,1,0,6))         // if name is empty, then it is set by kernel.
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
    let mut buf2 = [0u8; 512];
    let mut buf1 = [0u8; 512];
    let mut caddr: String = "null".to_string();
    loop {
        tokio::select! {
            _ = async {
                log::debug!("reading from interface");
                reader.read(&mut buf1).await;
                log::debug!("interface - {:?}", buf1);
            } => {
                    log::debug!("sending to client without addr");
                    if caddr != String::from("null") {
                        log::debug!("sending to client {}", &caddr);
                        socket.send_to(&buf1, &caddr).await; 
                    }        
                },
            _ = async {
                log::debug!("reading from client");
                let tup = socket.recv_from(&mut buf2).await;
                log::debug!("client - {:?}", buf2);
                match tup {
                    Ok((_amt, addr)) => {
                        caddr = addr.to_string();
                    }
                    Err(err) => {
                        log::error!("reading from client failed with error {}", err.to_string());
                    }
                }
            } => {
                    log::debug!("writting to interface");
                    writer.write(&mut buf2).await;
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