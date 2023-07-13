use clap::Parser;
use dns_parser::Packet as DnsPacket;
use fstrm::FstrmReader;
use log::{debug, info, warn, trace};
use ring_detection::{dnstap::Dnstap, socks::AutoRemoveFile};
use std::{
    io::Result,
    os::unix::net::{UnixListener, UnixStream},
    thread,
};

#[derive(Parser)]
#[command(name = "ring-detector")]
/// Works with your DNS server to detect when EZVIZ doorbell button is activated.
struct Cli {
    #[arg(short, long)]
    /// socket for dnstap listener
    socket: std::path::PathBuf,
}

fn handle_stream(stream: UnixStream) -> Result<()> {
    info!("connected to DNS server");
    let reader = FstrmReader::<_, ()>::new(stream);
    let mut reader = reader.accept()?.start()?;
    debug!("FSTRM handshake finish {:?}", reader.content_types());

    while let Some(mut frame) = reader.read_frame()? {
        let dnstap: Dnstap = protobuf::Message::parse_from_reader(&mut frame)?;
        let msg = dnstap.message.unwrap();
        match DnsPacket::parse(msg.query_message()) {
            Ok(packet) => handle_packet(packet),
            Err(err) => debug!("failed to parse DNS packet: {}", err),
        }
    }
    Ok(())
}

fn handle_packet(packet: DnsPacket) {
    let qtype_name = packet
        .questions
        .iter()
        .find(|q| {
            matches!(
                q.qtype,
                dns_parser::QueryType::A | dns_parser::QueryType::AAAA
            )
        })
        .map(|q| (q.qtype, q.qname.to_string()));
    trace!("name: {:?}", qtype_name);
}

fn main() {
    let cli = Cli::parse();

    let mut sock_path: AutoRemoveFile = cli.socket.to_str().expect("path is not valid").into();

    let listener = UnixListener::bind(&sock_path).expect("Cannot bind to socket");
    info!("listening on {}", sock_path);
    sock_path.set_auto_remove(true);

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
