use clap;
use clap::{App, Arg, SubCommand};

#[derive(Debug, Clone)]
pub struct Server {
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub remote_addr: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum Args {
    Info(Client),
    Client(Client),
    Server(Server),
}

pub fn get_args() -> Result<Args, String> {
    let matches = App::new("dprox")
        .version("1.0")
        .subcommand(
            SubCommand::with_name("server")
                .help("server mode")
                .arg(
                    Arg::with_name("bind")
                        .short("l")
                        .long("listen")
                        .default_value("0.0.0.0")
                        .help("set the listen address")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .default_value("8080")
                        .help("set the listen port")
                        .takes_value(true),
                )
        )
        .subcommand(
            SubCommand::with_name("client")
                .help("client mode")
                .arg(
                    Arg::with_name("server")
                        .short("s")
                        .long("server")
                        .help("set the remote server address")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("set the remote port")
                        .takes_value(true),
                )
        )
        .subcommand(
            SubCommand::with_name("info")
                .help("get info")
                .arg(
                    Arg::with_name("server")
                        .short("s")
                        .long("server")
                        .help("set the remote server address")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .help("set the remote port")
                        .takes_value(true),
                )
        )
        .get_matches();
    if let Some(matches) = matches.subcommand_matches("client") {
        let ip_str = matches
            .value_of("server")
            .ok_or_else(|| "can not find client host value")
            .unwrap();
        let port_str = matches
            .value_of("port")
            .ok_or_else(|| "can not find client port value")
            .unwrap();
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;
        Ok(Args::Client(Client {
            remote_addr: ip_str.to_string(),
            port: port,
        }))
    } else if let Some(matches) = matches.subcommand_matches("server") {
        let port_str = matches
            .value_of("port")
            .ok_or_else(|| "can not find server port value")
            .unwrap();
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;
        Ok(Args::Server(Server {
            port: port,
        }))
    } else if let Some(matches) =  matches.subcommand_matches("info") {
        let ip_str = matches
        .value_of("server")
        .ok_or_else(|| "can not find client host value")
        .unwrap();
        let port_str = matches
            .value_of("port")
            .ok_or_else(|| "can not find client port value")
            .unwrap();
        let port = port_str.parse::<u16>().map_err(|e| e.to_string())?;
        Ok(Args::Info(Client {
            remote_addr: ip_str.to_string(),
            port: port,
        }))
    } 
    else {
        unimplemented!()
    }
}
