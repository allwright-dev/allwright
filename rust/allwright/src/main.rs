use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// gRPC listen address for the engine server
    #[arg(long, default_value = "127.0.0.1:50051")]
    listen_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), tonic::transport::Error> {
    let args = Args::parse();
    println!("Starting engine gRPC server on {}", args.listen_addr);
    allwright::serve(args.listen_addr).await
}
