use clap;
use clap::{Args, Subcommand, Parser};

#[derive(Debug, Clone, Args)]
pub struct Server {
    #[arg(short, long, value_name = "Port", default_value="8080")]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Args)]
pub struct Client {
    #[arg(short, long, value_name = "IP")]
    pub remote_addr: String,
    #[arg(short, long, value_name = "Port", default_value="8080")]
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Role {
    /// To run dprox as server
    Server(Server),
    /// To run dprox as client
    Client(Client),
    /// To get network info of server
    Info(Client),
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct DproxArg {
    #[command(subcommand)]
    pub command: Role
}