use std::env;
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use opentelemetry_otlp::Protocol::Grpc;
use tonic::codegen::http::{HeaderMap, HeaderValue};
use tonic::metadata::MetadataMap;
use tonic::transport::Server;

use tracing_subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use aaru::consts::SYDNEY;
use aaru::server::route::router_service::router_server::RouterServer;
use aaru::server::route::{router_service, RouteService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Creating Router");
    let router = RouteService::from_file(SYDNEY).expect("-");
    tracing::info!("Router Created");

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(router_service::FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    // The remote server to log to...
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_tls_config(Default::default());

    // Initialize OpenTelemetry OLTP Protoc Pipeline
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Couldn't create OTLP tracer");

    // Link OTEL and STDOUT subscribers
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer();

    // Initialise tracing with subscribers and environment filter
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    let addr = "[::1]:9001".parse().unwrap();
    tracing::info!(message = "Starting server.", %addr);

    Server::builder()
        .trace_fn(|_| tracing::info_span!("aaru"))
        .add_service(RouterServer::new(router))
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}