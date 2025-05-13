use routers_codec::consts::LOS_ANGELES;
mod service;
mod trace;

use dotenv::dotenv;
use tonic::codegen::http::Method;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};

use crate::service::router_service::router_service_server::RouterServiceServer;
use crate::service::{RouteService, router_service};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load `.env` file
    dotenv()?;

    // Create the tracer first.
    trace::initialize_tracer();

    // Create the router
    tracing::info!("Creating Router");
    let router = RouteService::from_file(LOS_ANGELES).expect("-");

    // Initialize the reflector
    tracing::info!("Router Created");
    let reflector = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(router_service::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    // Set the address to serve from
    let addr = "[::1]:9001".parse().unwrap();
    tracing::info!(message = "Starting server.", %addr);

    Server::builder()
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any)
                // allow requests from any origin
                .allow_origin(Any),
        )
        .layer(GrpcWebLayer::new())
        .accept_http1(true)
        .tcp_nodelay(true)
        .add_service(RouterServiceServer::new(router))
        .add_service(reflector)
        .serve(addr)
        .await?;

    Ok(())
}
