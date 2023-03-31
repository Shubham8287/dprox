use log;
use std::{net::Ipv4Addr, str::from_utf8, u16};
use tokio::net::UdpSocket;
mod cli;
mod model;
use env_logger;
use rand::Rng;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn conn(turn_addr: String, port: u16) {
    log::info!("Starting as peer node");
    // separate octets from string ip
    let [a, b, c, d]: [u8; 4] = turn_addr
        .split('.')
        .map(|x| x.parse().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let mut node = model::PeerNode::new(Ipv4Addr::new(a, b, c, d), port);
    let turn = node.turn;
    let id = node.me.id;
    let mut rng = rand::thread_rng();
    let localport = rng.gen_range(9000..9100);
    tokio::task::spawn(async move {
        model::PeerNode::heartbeat(&turn, id, localport).await;
        });
    let tun = node.me.tun.as_mut();
    let (mut reader, mut writer) = tokio::io::split(tun.unwrap());
    let mut rng = rand::thread_rng();
    let localport = rng.gen_range(8000..9000);
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", localport))
        .await
        .expect("unbale to create socket");
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let raddr = turn_addr + ":" + &port.to_string();

    loop {
        tokio::select! {
            (len, sock_addr) = async {
                socket.recv_from(&mut buf2).await.expect("error receving from socket")
            } => {
                    log::info!("source {:?} -> interface", &sock_addr);
                    log::debug!("source {} -> interface: data {:?}", &sock_addr, &buf2[..len]);
                    writer.write(&mut buf2[0..len]).await.expect("error writting to interface");
            },
            len = async {
                reader.read(&mut buf1).await.expect("error reading to interface")
            } => {
                    log::info!("inteface -> address {} ", &raddr);
                    log::debug!("inteface -> address {}: data {:?}", &raddr, &buf1[..len]);
                    socket.send_to(&buf1[0..len], &raddr).await.expect("error  sending to socket");
            }
        };
    }
}

async fn serv(port: &u16) {
    log::info!("Starting as server node");
    let mut node = model::Turn::new();
    let map_ref = &mut (node.nodes);
    let tun = (node.me.tun).as_mut().unwrap();
    let (mut reader, mut writer) = tokio::io::split(tun);
    let socket = UdpSocket::bind(String::from("0.0.0.0:") + &port.to_string())
        .await
        .expect("unable to create socket");
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let mut connected_nodes: HashMap<u8, String> = HashMap::new();

    loop {
        tokio::select! {
            len = async {
                reader.read(&mut buf1).await.expect("error reading to interface")
            } => {
                    let client_node_id = buf1[19]; // last octet of receiver
                    if map_ref.contains_key(&client_node_id) {
                        let caddr = &map_ref[&client_node_id];
                        log::info!("inteface -> address {} ", &caddr);
                        log::debug!("inteface -> address {}: data {:?}", &caddr, &buf1[..len]);
                        socket.send_to(&buf1[0..len], &caddr).await.expect("error  sending to socket");
                    }
                },
            (len, sock_addr) = async {
                socket.recv_from(&mut buf2).await.expect("error receving from socket")
            } => {
                log::info!("source {:?} -> interface", &sock_addr);
                log::debug!("source {} -> interface: data {:?}", &sock_addr, &buf2[..len]);
                if buf2[0] == 1 {
                    socket.send_to(format!("{:?}",connected_nodes).as_bytes(), &sock_addr).await.expect("error sending to socket");
                } else if buf2[0] == 2 {
                    let id = buf2[1]; // last octet of sender
                    if id != 0 {
                        connected_nodes.insert(id, sock_addr.to_string());
                        }
                } else {
                writer.write(&mut buf2[0..len]).await.expect("error writting to interface");
                let id = buf2[15]; // last octet of sender
                if id != 0 {
                    map_ref.insert(id, sock_addr.to_string());
                    }
                }
            }
        };
    }
}

async fn get_info(turn_addr: String, port: u16) {
    log::info!("fetching network details");
    let socket = UdpSocket::bind("0.0.0.0:9090")
        .await
        .expect("unbale to create socket");
    socket
        .connect(format!("{}:{}", turn_addr, port))
        .await
        .expect("failed to connect to server");
    let mut buf = [0u8; 1200];
    buf[0] = 1;
    let _len = socket
        .send(&buf[0..100])
        .await
        .expect("unable to send get info request");
    log::info!("fetching network details");
    // recv from remote_addr
    let _len = socket
        .recv(&mut buf)
        .await
        .expect("failed while receving data");
    log::info!("fetching network details");
    println!("{:?}", from_utf8(&buf[.._len]).unwrap());
    drop(socket);
    // send to remote_addr
}

#[tokio::main]
async fn main() {
    env_logger::init();
    match cli::get_args().unwrap() {
        cli::Args::Client(client) => conn(client.remote_addr, client.port).await,
        cli::Args::Server(server) => serv(&server.port).await,
        cli::Args::Info(client) => get_info(client.remote_addr, client.port).await,
    }
}
