//! Creates the OpenTelemetry exporter using
//! `tracing_subscriber`, and batching requests
//! to the output defined by the environment.
//!
//! An example environment is shown:
//! ```bash
//! OTEL_EXPORTER_OTLP_ENDPOINT=https://<exporter>.com
//! OTEL_EXPORTER_OTLP_HEADERS=<api-key> [if required]
//! OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
//! OTEL_SERVICE_NAME=aaru
//! ```

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialises the tracer, using tracing subscription.
/// This is optional, not calling this function will simply
/// not log traces.
pub fn initialize_tracer() {
    // The remote server to log to...
    #[cfg(feature = "grpc_server")]
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_tls_config(Default::default());

    // Initialize OpenTelemetry OLTP Protoc Pipeline
    #[cfg(feature = "grpc_server")]
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Couldn't create OTLP tracer");

    // Link OTEL and STDOUT subscribers
    #[cfg(feature = "grpc_server")]
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer();

    // Initialise tracing with subscribers and environment filter
    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer);

    #[cfg(feature = "grpc_server")]
    registry.with(otel_layer).init();
    #[cfg(not(feature = "grpc_server"))]
    registry.init();
}