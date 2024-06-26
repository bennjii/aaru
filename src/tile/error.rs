use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bigtable_rs::bigtable::Error;
use prost::DecodeError;
use tracing::{error, event, Level, trace};

use crate::codec::mvt::Tile;

#[derive(Debug)]
pub enum TileError {
    BigTableError(bigtable_rs::bigtable::Error),
    AttachRepository(String),
    ProtoDecode(DecodeError),
    MissingEnvironment(String),
    NoTilesFound,
    UnsupportedZoom(u8),
    NoMatchingRepository
}

impl From<bigtable_rs::bigtable::Error> for TileError {
    fn from(value: Error) -> Self {
        Self::BigTableError(value)
    }
}

impl IntoResponse for TileError {
    fn into_response(self) -> Response {
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