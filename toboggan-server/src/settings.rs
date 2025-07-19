use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Settings {
    /// The host
    #[clap(long, env = "HOST", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub host: IpAddr,

    /// The port
    #[clap(long, env = "PORT", default_value_t = 8080)]
    pub port: u16,

    /// The talk to serve
    pub talk: PathBuf,
}
