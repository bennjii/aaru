use tonic::transport::Server;
use tracing::Level;
use tracing_subscriber;

use aaru::codec;
use aaru::consts::SYDNEY;
use aaru::server::route::router_service::router_server::RouterServer;
use aaru::server::route::{router_service, RouteService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(router_service::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // Initialise tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let addr = "[::1]:9001".parse().unwrap();
    let router = RouteService::from_file(SYDNEY).expect("-");

    tracing::info!(message = "Starting server.", %addr);

    Server::builder()
        .trace_fn(|v| tracing::info_span!("aaru"))
        .add_service(RouterServer::new(router))
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}