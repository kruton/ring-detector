use clap::Parser;
use log::{info, warn};
use ring_detection::socks::AutoRemoveFile;
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

fn handle_stream(_: UnixStream) -> Result<()> {
    info!("connected to DNS server");
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let mut sock_path: AutoRemoveFile = cli.socket.to_str().unwrap().into();

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
