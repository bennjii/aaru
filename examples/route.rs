use dotenv::dotenv;
use tonic::transport::Server;

use aaru::codec::consts::DISTRICT_OF_COLUMBIA;
use aaru::server::route::router_service::router_server::RouterServer;
use aaru::server::route::{router_service, RouteService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load `.env` file
    dotenv()?;

    // Create the tracer first.
    aaru::util::trace::initialize_tracer();

    // Create the router
    tracing::info!("Creating Router");
    let router = RouteService::from_file(DISTRICT_OF_COLUMBIA).expect("-");
    tracing::info!("Router Created");

    // Initialize the reflector
    let reflector = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(router_service::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // Set the address to serve from
    let addr = "[::1]:9001".parse().unwrap();
    tracing::info!(message = "Starting server.", %addr);

    Server::builder()
        .accept_http1(true)
        .trace_fn(|_| tracing::info_span!("aaru"))
        .add_service(RouterServer::new(router))
        .add_service(reflector)
        .serve(addr)
        .await?;

    Ok(())
}
