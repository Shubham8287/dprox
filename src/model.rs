
use log;
use std::{net::Ipv4Addr, u16};
use tokio::net::UdpSocket;
use rand::Rng;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashMap;
use tokio::time::{self, Duration};
use tokio_tun::{Tun, TunBuilder};
use std::sync::Arc;

pub struct Node {
    pub id: u8,
    pub tun: Option<Tun>,
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
pub struct PeerNode {
    pub me: Node,
    pub turn: (Ipv4Addr, u16),
}

impl PeerNode {
    pub fn new(turn_ip: Ipv4Addr, turn_port: u16) -> Self {
        let mut rng = rand::thread_rng();
        let last_oct: u8 = rng.gen_range(10..254);
        let ip = Ipv4Addr::new(10, 0, 0, last_oct.clone());
        let node = Node::new(last_oct, ip);
        PeerNode {
            me: node,
            turn: (turn_ip, turn_port),
        }
    }

    pub async fn heartbeat(turn: &(Ipv4Addr, u16), id: u8, socket: Arc<UdpSocket>) {
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

pub struct Turn {
    pub me: Node,
    pub nodes: HashMap<u8, String>,
}

impl Turn {
    pub fn new() -> Self {
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
