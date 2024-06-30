use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[cfg(feature = "tracing")]
use tracing::{event, Level};
use prost::DecodeError;

#[derive(Debug)]
pub enum TileError {
    DataSourceError(String),
    AttachRepository(String),
    ProtoDecode(DecodeError),
    MissingEnvironment(String),
    NoTilesFound,
    UnsupportedZoom(u8),
    NoMatchingRepository
}

impl IntoResponse for TileError {
    fn into_response(self) -> Response {
        #[cfg(feature = "tracing")]
        event!(Level::ERROR, name=?self);

        let code = match self {
            TileError::NoTilesFound => StatusCode::NO_CONTENT,
            TileError::NoMatchingRepository => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Response::builder()
            .status(code)
            .body(Body::from(()))
            .unwrap()
    }
}