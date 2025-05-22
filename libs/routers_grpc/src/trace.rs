//! Creates the OpenTelemetry exporter using
//! `tracing_subscriber`, and batching requests
//! to the output defined by the environment.
//!
//! An example environment is shown:
//! ```bash
//! OTEL_EXPORTER_OTLP_ENDPOINT=https://<exporter>.com
//! OTEL_EXPORTER_OTLP_HEADERS=<api-key> [if required]
//! OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
//! OTEL_SERVICE_NAME=routers
//! ```

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initialises the tracer, using tracing subscription.
/// This is optional, not calling this function will simply
/// not log traces.
pub fn initialize_tracer() {
    let otlp_exporter = SpanExporter::builder().with_tonic().build().unwrap();

    let tracer = SdkTracerProvider::builder()
        .with_simple_exporter(otlp_exporter)
        .build()
        .tracer("routers");

    // Link OTEL and STDOUT subscribers
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().compact();

    // Initialise tracing with subscribers and environment filter
    let registry = Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer)
        .with(otel_layer);

    registry.init();
}
