use clap::Parser;
use dns_parser::Packet as DnsPacket;
use fstrm::FstrmReader;
use log::{debug, info, warn};
use prost::{bytes::BytesMut, Message};
use ring_detection::dnstap::Dnstap;
use std::{
    io::{Read, Result},
    net::{Ipv4Addr, Ipv6Addr},
    os::unix::net::{UnixListener, UnixStream},
    thread,
};

static HOST_EU: &str = "alarm.eu.s3.amazonaws.com";
static HOST_US: &str = "alarm.use.s3.amazonaws.com";

#[derive(Parser)]
#[command(name = "ring-detector")]
/// Works with your DNS server to detect when EZVIZ doorbell button is activated.
struct Cli {
    #[arg(short, long)]
    /// socket for dnstap listener
    socket: std::path::PathBuf,

    #[arg(long)]
    /// MQTT hostname
    host: String,

    #[arg(long)]
    /// MQTT port
    port: u16,

    #[arg(short, long)]
    /// MQTT username
    username: String,

    #[arg(short, long)]
    /// MQTT password
    password: String,
}

#[derive(Debug)]
pub enum MyIpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl TryFrom<&[u8]> for MyIpAddr {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        match value.len() {
            4 => {
                let addr: [u8; 4] = value.try_into().unwrap();
                Ok(MyIpAddr::V4(Ipv4Addr::from(addr)))
            }
            16 => {
                let addr: [u8; 16] = value.try_into().unwrap();
                Ok(MyIpAddr::V6(Ipv6Addr::from(addr)))
            }
            _ => Err("Unexpected address length"),
        }
    }
}

fn handle_stream(stream: UnixStream) -> Result<()> {
    info!("connected to DNS server");
    let reader = FstrmReader::<_, ()>::new(stream);
    let mut reader = reader.accept()?.start()?;
    debug!("FSTRM handshake finish {:?}", reader.content_types());

    while let Some(mut frame) = reader.read_frame()? {
        // DataFrame appears to have some kind of bug where UnixStream does not return
        // 0 so it hangs until the next byte?
        let mut buffer = BytesMut::new();
        buffer.resize(frame.size(), 0);
        frame.read_exact(&mut buffer)?;

        let dnstap: Dnstap = Dnstap::decode(buffer)?;
        let msg = dnstap.message.expect("dnstap frame did not have message");

        let client: MyIpAddr = MyIpAddr::try_from(msg.query_address()).expect("invalid IP source");

        if let Some(query) = msg.query_message {
            match DnsPacket::parse(&query) {
                Ok(packet) => handle_packet(packet, client),
                Err(err) => debug!("failed to parse DNS packet: {}", err),
            }
        }
    }
    Ok(())
}

fn handle_packet(packet: DnsPacket, client: MyIpAddr) {
    let qtype_name = packet
        .questions
        .iter()
        .find(|q| {
            matches!(
                q.qtype,
                dns_parser::QueryType::A | dns_parser::QueryType::AAAA
            )
        })
        .map(|q| (q.qname.to_string(), &client));

    if let Some((hostname, _)) = &qtype_name {
        if hostname.eq_ignore_ascii_case(HOST_EU) || hostname.eq_ignore_ascii_case(HOST_US) {
            debug!("we got {} from {:?}", hostname, &client);
        }
    }
}

fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let cli = Cli::parse();

    if cli.socket.exists() {
        std::fs::remove_file(&cli.socket).expect("cannot delete existing socket");
    }

    let listener = UnixListener::bind(&cli.socket).expect("cannot bind to socket");
    info!("listening on {}", cli.socket.display());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || match handle_stream(stream) {
                    Ok(_) => info!("unbound disconnected"),
                    Err(err) => warn!("error on thread: {}", err),
                });
            }
            Err(err) => panic!("failure to connect: {}", err),
        }
    }
}
