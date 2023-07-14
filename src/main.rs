use clap::Parser;
use log::{info, warn};
use ring_detection::dns::{handle_packet, handle_stream};
use tokio::{net::UnixListener, task};

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

#[tokio::main]
async fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .init();

    let cli = Cli::parse();

    if cli.socket.exists() {
        std::fs::remove_file(&cli.socket).expect("cannot delete existing socket");
    }

    let listener = UnixListener::bind(&cli.socket).expect("cannot bind to socket");
    info!("listening on {}", cli.socket.display());

    tokio::select! {
        _ = async {
            loop {
                let stream = match listener.accept().await {
                    Ok((stream, _addr)) => stream,
                    Err(e) => panic!("failure to connect: {}", e),
                };

                task::spawn(async move {
                    match handle_stream(stream.into_std().unwrap(), handle_packet) {
                        Ok(_) => info!("unbound disconnected"),
                        Err(err) => warn!("error on thread: {}", err),
                    }
                });
            }
        } => {}
    }
}
