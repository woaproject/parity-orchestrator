#[macro_use] extern crate jsonrpc_client_core;
extern crate jsonrpc_client_http;

extern crate serde;
#[macro_use] extern crate serde_derive;

extern crate urlparse;
use urlparse::urlparse;

extern crate hex;

#[macro_use] extern crate slog;
extern crate slog_term;
extern crate slog_async;

extern crate reqwest;

extern crate config;
#[macro_use] extern crate clap;

extern crate byteorder;
#[macro_use] extern crate arrayref;

extern crate chrono;

mod myip;
mod as_hex;

#[derive(Serialize)]
pub enum Receiver {
    #[serde(rename = "public", with = "as_hex")]
    Public(Vec<u8>),
    #[serde(rename = "public", with = "as_hex")]
    Identity(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub struct Sender (
    #[serde(with = "as_hex")]
    Vec<u8>
);

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Binary (
    #[serde(with = "as_hex")]
    Vec<u8>
);

#[derive(Serialize)]
pub struct Post {
    to: Option<Receiver>,
    from: Option<Sender>,
    topics: Vec<Binary>,
    #[serde(with = "as_hex")]
    payload: Vec<u8>,
    padding: Option<Binary>,
    priority: u64,
    ttl: u64,
}

#[derive(Serialize)]
pub struct MessageFilter {
    #[serde(rename = "decryptWith")]
    decrypt_with: Option<Binary>,
    from: Option<Sender>,
    topics: Vec<Binary>,
}

#[derive(Deserialize)]
pub struct Message {
    from: Option<Sender>,
    recipient: Option<Sender>,
    ttl: u64,
    topics: Vec<Binary>,
    timestamp: u64,
    #[serde(with = "as_hex")]
    payload: Vec<u8>,
    padding: Option<Binary>,
}

#[derive(Deserialize)]
pub struct Peer {
    id: Option<String>,
}

#[derive(Deserialize)]
pub struct Peers {
    active: u64,
    connected: u64,
    max: u64,
    peers: Vec<Peer>,
}

mod client;
use client::ParityClient;

use jsonrpc_client_http::HttpTransport;

const DEFAULT_CONFIG: &str = "parity-orchestrator.toml";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum AddressConfig {
    #[serde(rename = "manual")]
    Manual {
        ip: String,
    },
    #[serde(rename = "aws_check_ip")]
    AWSCheckIP,
}

impl Default for AddressConfig {
    fn default() -> Self {
        AddressConfig::AWSCheckIP
    }
}

impl Into<Box<myip::IpAddressDiscovery>> for AddressConfig {
    fn into(self) -> Box<myip::IpAddressDiscovery> {
        match self {
            AddressConfig::Manual { ip } => Box::new(myip::StaticIP::new(ip)),
            AddressConfig::AWSCheckIP => Box::new(myip::AmazonCheckIP),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum TopicConfig {
    #[serde(rename = "binary")]
    Binary {
        #[serde(with = "as_hex")]
        topic: Vec<u8>,
    },
    #[serde(rename = "string")]
    String {
        topic: String,
    },
}

impl Into<Vec<u8>> for TopicConfig {
    fn into(self) -> Vec<u8> {
        match self {
            TopicConfig::Binary { topic } => topic.clone(),
            TopicConfig::String { topic } => topic.into(),
        }
    }
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    address: AddressConfig,
    node_announcement_topic: TopicConfig,
    // how often announcements should be sent out (in seconds)
    #[serde(default = "default_announcement_frequency")]
    node_announcement_frequency: u16,
    #[serde(default = "default_parity_node")]
    parity_node: String,
    #[serde(default = "default_reveal_trace_every_secs")]
    reveal_trace_every_secs: u64,
    #[serde(default = "default_orchestrator_additions_file")]
    orchestrator_additions_file: String,
}

fn default_parity_node() -> String {
    "http://localhost:8545".into()
}

fn default_announcement_frequency() -> u16 {
    30
}

fn default_reveal_trace_every_secs() -> u64 {
    10
}

fn default_orchestrator_additions_file() -> String {
    "parity-orchestrator-nodes".into()
}

use std::time::{Duration, Instant};
use std::fs;

fn main() {
    use slog::Drain;

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = slog::Logger::root(drain, o!());

    use clap::{Arg, App};

    let matches = App::new("Parity Orchestrator")
        .version(crate_version!())
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Sets a custom config file")
            .default_value(DEFAULT_CONFIG)
            .takes_value(true))
        .get_matches();

    let config_file = matches.value_of("config").unwrap();

    if !::std::path::Path::new(config_file).exists() {
        if config_file != DEFAULT_CONFIG {
            error!(log, "config file {} not found, aborting", config_file);
            drop(log); // ensure the log message propagates
            ::std::process::exit(1);
        } else {
            warn!(log, "default config {} not found", config_file);
        }
    }


    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name(config_file).required(false)).unwrap();

    let config: Config = match settings.try_into() {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(log, "Failed loading config, aborting: {:?}", err);
            drop(log);
            ::std::process::exit(1);
        }

    };

    info!(log, "Parity Orchestrator started");

    // prepare additions file so it can be monitored from the very beginning
    let mut f = fs::OpenOptions::new().create(true).append(true).open(&config.orchestrator_additions_file).expect("can't open additions file");
    drop(f);

    let transport = HttpTransport::new().unwrap();
    let transport_handle = transport.handle(config.parity_node.as_str()).unwrap();
    let mut client = ParityClient::new(transport_handle);

    let enode = loop {
        match client.parity_enode().call() {
            Ok(result) => {
                break result;
            },
            Err(_) => {
                ::std::thread::sleep(Duration::from_secs(1));
                info!(log, "Waiting for Parity to become available");
            }
        }
    };
    info!(log, "Connected to enode {}", enode);
    let node_id = urlparse(enode).username.expect("cna't fetch node id");
    info!(log, "Node ID: {}", node_id);
    let node_id_bin: Vec<u8> = hex::decode(&node_id).expect("can't parse node id as a hexadecimal binary");

    info!(log, "IP discovery mechanism: {:?}", config.address);
    let ip_discovery: Box<myip::IpAddressDiscovery> = config.address.clone().into();
    let ip = ip_discovery.discover_ip_address().expect("can't discover IP address");
    info!(log, "Local IP detected: {}", ip);

    let port = client.parity_netPort().call().expect("can't fetch port Parity is listening on");
    info!(log, "Listening on port: {}", port);

    let callback = loop {
        let cb = ::std::net::TcpStream::connect_timeout(&::std::net::SocketAddr::new(ip, port), Duration::from_secs(10));

        match cb {
            Ok(_) => {
                info!(log, "Callback to {}:{} has been successful", ip, port);
                break cb;
            },
            Err(err) => {
                error!(log, "Callback to {}:{} has been unsuccessful: {:?}, waiting for it to become available", ip, port, err);
                ::std::thread::sleep(Duration::from_secs(1));
            }
        }

    };

    // Subscribe to the announcement topic
    let announcement_topic: Vec<u8> = config.node_announcement_topic.clone().into();

    let filter = client.shh_newMessageFilter(MessageFilter {
        decrypt_with: None,
        from: None,
        topics: vec![Binary(announcement_topic.clone())],
    }).call().expect("can't subscribe to announcements");

    // prepare announcement message
    let mut announcement = vec![];
    use std::io::Write;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};
    announcement.write(&node_id_bin).unwrap();
    match ip {
        IpAddr::V4(v4) => announcement.write(&v4.octets()).unwrap(),
        IpAddr::V6(v6) => announcement.write(&v6.octets()).unwrap(),
    };
    announcement.write_u16::<BigEndian>(port).unwrap();


    let res = client.shh_post(Post {
        to: None,
        from: None,
        topics: vec![Binary(announcement_topic.clone())],
        payload: announcement.clone(),
        padding: None,
        priority: 100,
        ttl: 60,
    }).call().expect("can't send an announcement request");

    if !res {
        error!(log, "Failed sending an announcement");
    }

    let mut last_announcement = Instant::now();
    let node_announcement_frequency = Duration::from_secs(config.node_announcement_frequency as u64);

    let mut reveal_instant = Instant::now();
    loop {
        ::std::thread::sleep(Duration::from_secs(1));
        if config.reveal_trace_every_secs > 0 && reveal_instant.elapsed().as_secs() >= config.reveal_trace_every_secs {
            reveal_instant = Instant::now();
            info!(log, "Polling announcements (reveal_trace_every_secs = {})", config.reveal_trace_every_secs);
        } else {
            trace!(log, "Polling announcements");
        }
        let messages = client.shh_getFilterMessages(filter.clone()).call().expect("failed to poll announcements");

        if messages.len() > 0 {
            info!(log, "Got {} message(s)", messages.len());
        }

        for message in messages {
            let payload = message.payload;
            let peer = &payload[0..64];
            let peer_id = hex::encode(peer);
            let address: Result<(IpAddr, u16), ::std::io::Error> = match payload.len() - 64 {
                6 => { // ip v4 : port
                    let ip = IpAddr::V4(Ipv4Addr::from(*array_ref![payload, 64, 4]));
                    match (&payload[68..]).read_u16::<BigEndian>() {
                        Ok(port) => Ok((ip, port)),
                        Err(err) => Err(err),
                    }
                },
                16 => {  // ip v6 : port
                    let ip = IpAddr::V6(Ipv6Addr::from(*array_ref![payload,64,16]));
                     match (&payload[68..]).read_u16::<BigEndian>() {
                        Ok(port) => Ok((ip, port)),
                        Err(err) => Err(err),
                    }
                },
                other => Err(::std::io::Error::new(::std::io::ErrorKind::InvalidInput, format!("invalid input size {}", other))),
            };
            match address {
                Ok(addr) => {
                    trace!(log, "Read announcement: {:?}", addr);
                },
                Err(err) => {
                    warn!(log, "Can't parse announcement: {:?}", err);
                    break;
                }
            }
            let res = client.parity_netPeers().call();
            match res {
                Err(err) => error!(log, "Can't fetch peers: {}", err),
                Ok(peers) => {
                    let peer_found = peers.peers.into_iter().any(|p| p.id.and_then(|v| Some(v == peer_id)).or_else(|| Some(false)).unwrap());
                    if !peer_found {
                        let (ip, port) = address.unwrap();
                        let enode = format!("enode://{}@{}:{}", peer_id, ip, port);
                        info!(log, "Adding new peer: {}", enode);
                        let res = client.parity_addReservedPeer(enode.clone()).call();
                        if res.is_err() {
                            error!(log, "Can't add new peer: {}", res.unwrap_err());
                        } else {
                            use std::fs;
                            use chrono::prelude::*;
                            let mut f = fs::OpenOptions::new().create(true).append(true).open(&config.orchestrator_additions_file).expect("can't open additions file");
                            f.write(format!("{} {}\n", Utc::now().to_rfc3339(),enode).as_bytes()).expect("can't write to the additions file");
                            if !res.unwrap() {
                                error!(log, "Parity refused adding this new peer");
                            }
                        }
                    } else {
                        trace!(log, "Peer found, skipping");
                    }
                }
            }
        }


        if last_announcement.elapsed() >= node_announcement_frequency {
            last_announcement = Instant::now();
            let res = client.shh_post(Post {
                to: None,
                from: None,
                topics: vec![Binary(announcement_topic.clone())],
                payload: announcement.clone(),
                padding: None,
                priority: 100,
                ttl: 60,
            }).call().expect("can't send an announcement request");

            if !res {
                error!(log, "Failed sending an announcement");
            }

        }

    }

}
