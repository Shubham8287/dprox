use clap::Parser;
use log;
use std::{net::Ipv4Addr, str::from_utf8, u16};
use tokio::net::UdpSocket;
mod cli;
mod model;
use env_logger;
use rand::Rng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;

async fn conn(turn_addr: &String, port: u16) {
    log::info!("Starting as peer node");
    
    log::info!("Setting Tun");
    let mut node = model::PeerNode::new(turn_addr.parse::<Ipv4Addr>().unwrap(), port);
    let turn = node.turn;
    let id = node.me.id;
    let tun = node.me.tun.as_mut();
    let (mut reader, mut writer) = tokio::io::split(tun.unwrap());

    log::info!("setting socket");
    let mut rng = rand::thread_rng();
    let localport = rng.gen_range(8000..9000);
    let socket = Arc::new(UdpSocket::bind(("0.0.0.0", localport)).await.expect("unbale to create socket"));
    let cloned_socket = Arc::clone(&socket);

    log::info!("spawning healthcheck");
    let _healthcheck_task = tokio::task::spawn(async move {
        model::PeerNode::heartbeat(&turn, id, cloned_socket).await;
        });

    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let raddr = turn_addr.to_owned() + ":" + &port.to_string();
    //_healthcheck_task.await.unwrap();
    loop {
        tokio::select! {
            (len, sock_addr) = async {
                socket.recv_from(&mut buf2).await.expect("error receving from socket")
            } => {
                    log::info!("source {:?} -> interface", sock_addr);
                    log::debug!("source {} -> interface: data {:?}", sock_addr, &buf2[..len]);
                    writer.write(&mut buf2[0..len]).await.expect("error writting to interface");
            },
            len = async {
                reader.read(&mut buf1).await.expect("error reading to interface")
            } => {
                    log::info!("inteface -> address {} ", raddr);
                    log::debug!("inteface -> address {}: data {:?}", raddr, &buf1[..len]);
                    socket.send_to(&buf1[0..len], &raddr).await.expect("error sending to socket");
            }
        };
    }

}



async fn serv(port: u16) {
    log::info!("Starting as server node");

    log::info!("Setting tun");
    let mut node = model::Turn::new();
    let map_ref = &mut (node.nodes);
    let tun = (node.me.tun).as_mut().unwrap();
    let (mut reader, mut writer) = tokio::io::split(tun);

    log::info!("Setting socket");
    let socket = UdpSocket::bind(("0.0.0.0", port))
        .await
        .expect("unable to create socket");
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];

    log::info!("creating connected_nodes to keep active connections");

    loop {
        tokio::select! {
            len = async { reader.read(&mut buf1).await.expect("Error reading from interface") } => {
                let client_node_id = buf1[19]; // Last octet of receiver
                if let Some(caddr) = map_ref.get(&client_node_id) {
                    log::info!("Interface -> Address {}", &caddr);
                    log::debug!("Interface -> Address {}: data {:?}", &caddr, &buf1[..len]);
                    socket.send_to(&buf1[..len], &caddr).await.expect("Error sending to socket");
                }
            },
            (len, sock_addr) = async { socket.recv_from(&mut buf2).await.expect("Error receiving from socket") } => {
                log::info!("Source {:?} -> Interface", &sock_addr);
                log::debug!("Source {} -> Interface: data {:?}", &sock_addr, &buf2[..len]);
                
                match buf2[0] {
                    1 => {
                        socket.send_to(format!("{:?}", map_ref).as_bytes(), &sock_addr).await.expect("Error sending to socket");
                    },
                    2 => {
                        let id = buf2[1]; // Last octet of sender
                        if id != 0 {
                            map_ref.insert(id, sock_addr.to_string());
                        }
                    },
                    _ => {
                        writer.write(&buf2[..len]).await.expect("Error writing to interface");
                        // let id = buf2[15]; // Last octet of sender
                        // if id != 0 {
                        //     map_ref.insert(id, sock_addr.to_string());
                        // }
                    }
                }
            }
        };
    }
}

async fn get_info(turn_addr: &String, port: u16) {
    log::info!("Fetching network details...");
    
    let socket = UdpSocket::bind("0.0.0.0:9090")
        .await
        .expect("Failed to create socket");
    
    match socket.connect(format!("{}:{}", turn_addr, port)).await {
        Ok(_) => log::info!("Connected to server"),
        Err(e) => {
            log::error!("Failed to connect to server: {}", e);
            return;
        }
    }
    
    let mut buf = [0u8; 1200];
    buf[0] = 1;
    
    match socket.send(&buf[..100]).await {
        Ok(len) => log::info!("Sent {} bytes to server", len),
        Err(e) => {
            log::error!("Failed to send request to server: {}", e);
        }
    }
    
    match socket.recv(&mut buf).await {
        Ok(len) => {
            log::info!("Received {} bytes from server", len);
            println!("{:?}", std::str::from_utf8(&buf[..len]).unwrap());
        },
        Err(e) => {
            log::error!("Failed to receive data from server: {}", e);
        }
    }
    
    drop(socket);
    log::info!("Closed connection to server");
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = cli::DproxArg::parse();
    match &args.command {
        cli::Role::Client(client) => conn(&client.remote_addr, client.port.unwrap()).await,
        cli::Role::Server(server) => serv(server.port.unwrap()).await,
        cli::Role::Info(client) => get_info(&client.remote_addr, client.port.unwrap()).await,
    }
}
