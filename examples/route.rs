use dotenv::dotenv;
use tonic::codegen::http::Method;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use tower_http::cors::{Any, CorsLayer};
use aaru::codec::consts::SYDNEY;
use aaru::server::route::{router_service, RouteService};
use aaru::server::route::router_service::router_service_server::RouterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load `.env` file
    dotenv()?;

    // Create the tracer first.
    #[cfg(feature = "tracing")]
    aaru::util::trace::initialize_tracer();

    // Create the router
    #[cfg(feature = "tracing")]
    tracing::info!("Creating Router");
    let router = RouteService::from_file(SYDNEY).expect("-");
   
    #[cfg(feature = "tracing")]
    tracing::info!("Router Created");

    // Initialize the reflector
    let reflector = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(router_service::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    // Set the address to serve from
    let addr = "[::1]:9001".parse().unwrap();
    #[cfg(feature = "tracing")]
    tracing::info!(message = "Starting server.", %addr);

     Server::builder()
         .layer(
             CorsLayer::new()
                 .allow_methods([Method::GET, Method::POST])
                 .allow_headers(Any)
                 // allow requests from any origin
                 .allow_origin(Any)
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
