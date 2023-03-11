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
    nodes: Vec<(Node, i32)>
}

impl Turn {
    fn new(id: &str) -> Self {
        let ip = Ipv4Addr::new(10,0,0,5);
        let node = Node::new(id, ip);
        Turn {me: node, nodes: Vec::new()}
    }

    fn add_node( &mut self, node: Node) {
        self.nodes.push((node, 10));
    }
}


async fn conn (turn_addr: String, port: u16) {
    log::info!("Starting as peer node");
    // separate octets from string ip
    let [a,b,c,d] : [u8; 4] = turn_addr.split('.')
        .map(|x| x.parse().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let node = PeerNode::new("hostname", Ipv4Addr::new(a,b,c,d), port);

    let (mut reader, mut writer) = tokio::io::split(node.me.tun.unwrap());
    let socket = UdpSocket::bind("0.0.0.0:8080").await.expect("unbale to create socket");
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

async fn serv (port: &u16)  {
    log::info!("Starting as server node");

    let node = Turn::new("turn");
    let (mut reader, mut writer) = tokio::io::split(node.me.tun.unwrap());

    let socket = UdpSocket::bind(String::from("0.0.0.0:") + &port.to_string()).await;
    let socket = UdpSocket::bind("0.0.0.0:8080").await.expect("unbale to create socket");
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];
    let mut caddr: String = "null".to_string();

    loop {
        tokio::select! {
            len = async {
                reader.read(&mut buf1).await.expect("error reading to interface")
            } => {
                    log::debug!("sending to client without addr");
                    if caddr != String::from("null") {
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
                writer.write(&mut buf2[0..len]).await.expect("error writting to interface");
                caddr = sock_addr.to_string();
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