//! MVT (Mapbox Vector Tile)
//!
//! Contains the required implementation logic to convert
//! tile data from `Value` items into a `Tile`.

use axum::http::{HeaderValue, StatusCode};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use prost::Message;

use crate::codec::mvt::{Layer, Tile, Value};

const MVT_CONTENT_TYPE: &str = "application/vnd.mapbox-vector-tile";

impl From<Vec<Layer>> for Tile {
    fn from(value: Vec<Layer>) -> Self {
        Self {
            layers: value,
        }
    }
}

impl From<Layer> for Tile {
    fn from(value: Layer) -> Self {
        Tile::from(vec![value])
    }
}

impl Value {
    pub fn from_int(value: i64) -> Self {
        Self {
            int_value: Some(value),
            ..Self::default()
        }
    }

    pub fn from_float(float: f32) -> Self {
        Self {
            float_value: Some(float),
            ..Self::default()
        }
    }

    pub fn from_string(value: String) -> Self {
        Self {
            string_value: Some(value),
            ..Self::default()
        }
    }
}

impl IntoResponse for Tile {
    fn into_response(self) -> Response {
        let mut res = (StatusCode::OK, self.encode_to_vec()).into_response();

        // Attach MVT content type in header
        match HeaderValue::from_str(MVT_CONTENT_TYPE) {
            Err(_) => {}
            Ok(value) => {
                res.headers_mut().append(CONTENT_TYPE, value);
            }
        }

        res
    }
}