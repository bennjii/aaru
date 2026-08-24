//! Error type for the Overture codec.

use core::fmt;

/// Errors raised while reading or assembling an Overture network.
#[derive(Debug)]
pub enum OvertureError {
    /// Filesystem / IO failure.
    Io(std::io::Error),
    /// No `*.parquet` files were found at the given path.
    NoParquetFiles(String),
    /// A GeoParquet column was missing or had an unexpected type.
    Schema(String),
    /// A record could not be decoded (Arrow, WKB, or serde_arrow failure).
    Decode(String),
}

impl fmt::Display for OvertureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OvertureError::Io(e) => write!(f, "io error: {e}"),
            OvertureError::NoParquetFiles(p) => write!(f, "no parquet files found at `{p}`"),
            OvertureError::Schema(m) => write!(f, "geoparquet schema error: {m}"),
            OvertureError::Decode(m) => write!(f, "decode error: {m}"),
        }
    }
}

impl core::error::Error for OvertureError {}

impl From<std::io::Error> for OvertureError {
    fn from(value: std::io::Error) -> Self {
        OvertureError::Io(value)
    }
}
