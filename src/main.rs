use log;
use std::{net::Ipv4Addr, str::from_utf8, u16};
use tokio::net::UdpSocket;
mod cli;
use env_logger;
use rand::Rng;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{self, Duration};
use tokio_tun::{Tun, TunBuilder};

struct Node {
    id: u8,
    tun: Option<Tun>,
}

impl Node {
    fn new(id: u8, ip: Ipv4Addr) -> Node {
        let tun = TunBuilder::new()
            .name("")
            .address(ip)
            .netmask(Ipv4Addr::new(255, 255, 255, 0)) // if name is empty, then it is set by kernel.
            .tap(false) // false (default): TUN, true: TAP.
            .packet_info(false) // false: IFF_NO_PI, default is true.
            .up() // or set it up manually using `sudo ip link set <tun-name> up`.
            .try_build();

        let tun = match tun {
            Ok(tun) => {
                log::info!("initialized Node with Tun {:?}", ip);
                tun
            }
            Err(_err) => {
                panic!("failed to intialized tun with error {:?}", _err)
            }
        };

        Node {
            id: id,
            tun: Some(tun),
        }
    }
}
struct PeerNode {
    me: Node,
    turn: (Ipv4Addr, u16),
}

impl PeerNode {
    fn new(turn_ip: Ipv4Addr, turn_port: u16) -> Self {
        let mut rng = rand::thread_rng();
        let last_oct: u8 = rng.gen_range(10..254);
        let ip = Ipv4Addr::new(10, 0, 0, last_oct.clone());
        let node = Node::new(last_oct, ip);
        PeerNode {
            me: node,
            turn: (turn_ip, turn_port),
        }
    }

    async fn heartbeat(turn: &(Ipv4Addr, u16), id: u8, localport :u16) {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", localport)).await.expect("unbale to create socket");
        let mut interval = time::interval(Duration::from_secs(3));
        let (ip, port) = turn;
        socket
            .connect(format!("{}:{}", ip.to_string(), port))
            .await
            .expect("failed to connect to server");
        let mut buf = [0u8; 120];
        buf[0] = 2;
        buf[1] = id;
        loop {
            socket.send(&buf).await.expect("unable to send heartbeat");
            interval.tick().await;
        }
    }
}

struct Turn {
    me: Node,
    nodes: HashMap<u8, String>,
}

impl Turn {
    fn new() -> Self {
        let ip = Ipv4Addr::new(10, 0, 0, 5);
        let node = Node::new(5, ip);
        let mut node_map: HashMap<u8, String> = HashMap::new();
        node_map.insert(5, ip.to_string());
        Turn {
            me: node,
            nodes: node_map,
        }
    }

    fn add_node(&mut self, id: u8, node_addr: &String) {
        self.nodes.insert(id, node_addr.clone());
    }
}

impl Serialize for Turn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Turn", 2)?;
        state.serialize_field("me", &self.me.id)?;
        state.serialize_field("nodes", &self.nodes)?;

        state.end()
    }
}

async fn conn(turn_addr: String, port: u16) {
    log::info!("Starting as peer node");
    // separate octets from string ip
    let [a, b, c, d]: [u8; 4] = turn_addr
        .split('.')
        .map(|x| x.parse().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let mut node = PeerNode::new(Ipv4Addr::new(a, b, c, d), port);
    let turn = node.turn;
    let id = node.me.id;
    let mut rng = rand::thread_rng();
    let localport = rng.gen_range(9000..9100);
    tokio::task::spawn(async move {
        PeerNode::heartbeat(&turn, id, localport).await;
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

    let mut node = Turn::new();
    let map_ref = &mut (node.nodes);
    let tun = (node.me.tun).as_mut().unwrap();
    let (mut reader, mut writer) = tokio::io::split(tun);
    let socket = UdpSocket::bind(String::from("0.0.0.0:") + &port.to_string())
        .await
        .expect("unable to create socket");
    let mut buf1 = [0u8; 1500];
    let mut buf2 = [0u8; 1500];

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
                    socket.send_to(format!("{:?}",map_ref).as_bytes(), &sock_addr).await.expect("error sending to socket");
                } else if buf2[0] == 2 {
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
