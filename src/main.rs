use clap::Parser;

#[derive(Parser)]
#[command(name = "ring-detector")]
/// Works with your DNS server to detect when EZVIZ doorbell button is activated.
struct Cli {
    #[arg(short, long)]
    /// socket for dnstap listener
    socket: std::path::PathBuf,
}

fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli.socket);
}
