use axum::extract::State;
use axum::http::{header, Method};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{serve, Router};
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer, MaxAge};

use aaru::tile::repositories::RepositorySet;
use axum::http::StatusCode;
use futures::join_all;
use tracing::{event, Level};

async fn health_check(State(state): State<Arc<RepositorySet>>) -> Response {
    let futures: Vec<_> = state
        .repositories
        .iter()
        .map(|(id, repo)| {
            event!(Level::DEBUG, name = "repo::ping", ?id);
            repo.ping()
        })
        .collect();

    let results = join_all(futures).await;

    for result in results {
        if let Err(response) = result.map_err(|e| aaru::Error::Tile(e)) {
            return response.into_response();
        }
    }

    StatusCode::OK.into_response()
}

fn cors(origins: String) -> CorsLayer {
    CorsLayer::new()
        .allow_methods(vec![Method::GET])
        .allow_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_origin(AllowOrigin::list(
            origins
                .split(",")
                .map(|o| o.parse())
                .filter_map(|o| match o {
                    Err(_) => None,
                    Ok(o) => Some(o),
                }),
        ))
        .max_age(MaxAge::exact(Duration::new(3600, 0)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load `.env` file
    dotenv()?;

    let port: u16 = env::var("APP_PORT").expect("").parse().expect("");

    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .or::<&str>(Ok("".to_string()))
        .unwrap();

    // Create the tracer first.
    aaru::util::trace::initialize_tracer();

    // Set the address to serve from
    let addr = tokio::net::TcpListener::bind(format!("localhost:{port}")).await?;
    tracing::info!(message = "Starting server.", ?addr);

    let big_table = init_bq().await.expect("Could not initialize BigTable");

    let state = RepositorySet::new().attach(big_table, "big_table");

    let app = Router::new()
        .route("/", get(health_check))
        .layer(cors(allowed_origins))
        .with_state(Arc::new(state));

    serve(addr, app).await?;

    tracing::info!(message = "Terminating server.");
    Ok(())
}
