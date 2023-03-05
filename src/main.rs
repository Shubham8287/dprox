use tokio::net::UdpSocket;
use std::{net::{Ipv4Addr, SocketAddr, IpAddr}, u16};
use log;
mod cli;
use env_logger;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio_tun::{TunBuilder, Tun};
use rand::Rng;

struct Node {
    id: String,
    tun: Option<Tun>
}

impl Node {
    fn new(id: &str, ip: Ipv4Addr) -> Node {
        let tun = TunBuilder::new()
        .name("")
        .address(ip)
        .netmask(Ipv4Addr::new(255,255,255,0))         // if name is empty, then it is set by kernel.
        .tap(false)   // false (default): TUN, true: TAP.
        .packet_info(false)  // false: IFF_NO_PI, default is true.
        .up()                // or set it up manually using `sudo ip link set <tun-name> up`.
        .try_build();

        let tun = match tun {
            Ok(tun) => {
                log::info!("initialized Node with Tun {:?}", ip);
                tun
            },
            Err(_err) => { panic!("failed to intialized tun with error {:?}", _err) }
        };
        
        Node {id: id.to_string(), tun: Some(tun)}
    }
}
struct PeerNode {
    me: Node,
    turn: (Ipv4Addr, u16),
}

impl PeerNode {
    fn new(id: &str, turn_ip: Ipv4Addr, turn_port: u16) -> Self {
        let mut rng = rand::thread_rng();
        let ip = Ipv4Addr::new(10,0,0,rng.gen_range(10..254));
        let node = Node::new(id, ip);
        PeerNode {me: node, turn: (turn_ip, turn_port)}
    }

    fn broadcast() {
        unimplemented!()
    }
}

struct Turn {
    me: Node,
    nodes: Vec<Node>
}

impl Turn {
    fn new(id: &str) -> Self {
        let ip = Ipv4Addr::new(10,0,0,5);
        let node = Node::new(id, ip);
        Turn {me: node, nodes: Vec::new()}
    }
    fn add_node( &mut self, node: Node) {
        self.nodes.push(node);
    }
}


async fn conn (turn_addr: String, port: u16) {
    log::info!("Starting as peer node");
    let [a,b,c,d] : [u8; 4] = turn_addr.split('.')
        .map(|x| x.parse().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();
    let node = PeerNode::new("hostname", Ipv4Addr::new(a,b,c,d), port);

    let (mut reader, mut writer) = tokio::io::split(node.me.tun.unwrap());
    let socket = UdpSocket::bind("0.0.0.0:8080").await;
    let socket = match socket {
        Ok(socket) => socket,
        Err(_err) => { panic!("yo") }
    };
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let raddr = _addr + ":" + &port.to_string();
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
    log::info!("Starting as server node");
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
  //  let mut buf1 = [0u8; 1500];
    let mut caddr: String = "null".to_string();
    let mut len1 = 0;
    loop {
        tokio::select! {
            (buf, len) = async {
                let mut buf = [0u8; 1500];
                let mut len = 0;
                log::debug!("reading from interface");
                len = reader.read(&mut buf).await.expect("error while reading from interface");
                log::debug!("interface - {:?}", &buf[0..len]);
                (buf[0..len].to_vec(), len) } => {
                    log::debug!("sending to client without addr");
                    if caddr != String::from("null") {
                        log::debug!("sending to client {}", &caddr);
                        socket.send_to(&buf[0..len], &caddr).await.expect("error in writting to socket"); 
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
}