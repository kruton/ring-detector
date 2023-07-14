use clap::Parser;
use log::{info, warn};
use ring_detection::dns::{handle_packet, handle_stream};
use std::{os::unix::net::UnixListener, thread};

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
                thread::spawn(move || match handle_stream(stream, handle_packet) {
                    Ok(_) => info!("unbound disconnected"),
                    Err(err) => warn!("error on thread: {}", err),
                });
            }
            Err(err) => panic!("failure to connect: {}", err),
        }
    }
}
