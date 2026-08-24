mod runtime;

use clap::{Parser, Subcommand};
use std::error::Error;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<WebCommand>,
}

#[derive(Subcommand, Debug)]
enum WebCommand {
    /// Start the web surface runtime server
    Serve {
        /// gRPC listen address for the engine server
        #[arg(long, default_value = "127.0.0.1:50051")]
        listen_addr: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command.unwrap_or(WebCommand::Serve {
        listen_addr: "127.0.0.1:50051".parse()?,
    }) {
        WebCommand::Serve { listen_addr } => runtime::serve(listen_addr).await?,
    }

    Ok(())
}
