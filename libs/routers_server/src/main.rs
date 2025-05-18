pub mod lib;
pub mod services;
pub mod trace;

use crate::lib::proto;
use std::sync::Arc;

use crate::lib::r#match::MatchServiceServer;
use crate::lib::optimise::OptimisationServiceServer;
use crate::lib::proximity::ProximityServiceServer;

use crate::services::RouteService;

use dotenv::dotenv;
use fixtures::{LOS_ANGELES, fixture};
use tonic::codegen::http::Method;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load `.env` file
    dotenv()?;

    // Create the tracer first.
    trace::initialize_tracer();

    // Create the router
    tracing::info!("Creating Router");
    let router_base = RouteService::from_file(fixture!(LOS_ANGELES).to_str().unwrap()).expect("-");

    let router = Arc::new(router_base);

    // Initialize the reflector
    tracing::info!("Router Created");
    let reflector = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
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
        .add_service(OptimisationServiceServer::new(router.clone()))
        .add_service(MatchServiceServer::new(router.clone()))
        .add_service(ProximityServiceServer::new(router.clone()))
        .add_service(reflector)
        .serve(addr)
        .await?;

    Ok(())
}
