pub mod server;

use std::io::Read;
use std::net::UdpSocket;

use tun::platform::Device;

extern crate tun;

fn create_tun() -> Device {
	let mut config = tun::Configuration::default();
	config.address((10, 0, 0, 1))
	       .netmask((255, 255, 255, 0))
	       .up();

	#[cfg(target_os = "linux")]
	config.platform(|config| {
		config.packet_information(true);
	});

	tun::create(&config).unwrap()
}

fn main() {
    let mut dev = create_tun();
	let mut buf = [0; 4096];

    let socket = UdpSocket::bind("127.0.0.1:34254").expect("couldn't bind to address");

	loop {
		let amount = dev.read(&mut buf).unwrap();
		println!("received from tap is - {:?}", &buf[0 .. amount]);
        socket.send_to(&buf, "127.0.0.1:8081");
	}
}