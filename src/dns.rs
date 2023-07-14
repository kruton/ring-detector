use super::{dnstap::Dnstap, net::parse_octets};
use anyhow::Result;
use dns_parser::Packet as DnsPacket;
use fstrm::FstrmReader;
use log::{debug, info};
use prost::{bytes::BytesMut, Message};
use std::{io::Read, net::IpAddr, os::unix::net::UnixStream};

const HOST_EU: &str = "alarm.eu.s3.amazonaws.com";
const HOST_US: &str = "alarm.use.s3.amazonaws.com";

type PacketHandler = fn(DnsPacket, IpAddr) -> Result<()>;

pub fn handle_stream(stream: UnixStream, handler: PacketHandler) -> Result<()> {
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

        handle_frame(buffer, handler)?;
    }
    Ok(())
}

fn handle_frame(buffer: BytesMut, handler: PacketHandler) -> Result<()> {
    let dnstap: Dnstap = Dnstap::decode(buffer)?;
    let msg = dnstap.message.expect("dnstap frame did not have message");

    let client: IpAddr = parse_octets(msg.query_address()).expect("invalid IP source");

    if let Some(query) = msg.query_message {
        match DnsPacket::parse(&query) {
            Ok(packet) => handler(packet, client)?,
            Err(e) => {
                debug!("failed to parse DNS packet: {}", e);
                ()
            }
        }
    }

    Ok(())
}

pub fn handle_packet(packet: DnsPacket, client: IpAddr) -> Result<()> {
    let _: Vec<_> = packet
        .questions
        .iter()
        .filter(|q| {
            matches!(
                q.qtype,
                dns_parser::QueryType::A | dns_parser::QueryType::AAAA
            )
        })
        .filter_map(|q| {
            let name = &q.qname;
            match name.to_string().as_str() {
                HOST_EU | HOST_US => {
                    debug!("we got {} from {:?}", name, &client);
                    Some(name)
                }
                _ => None,
            }
        })
        .collect();

    Ok(())
}
