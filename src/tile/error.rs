use bigtable_rs::bigtable::Error;
use prost::DecodeError;

#[derive(Debug)]
pub enum TileError {
    BigTableError(bigtable_rs::bigtable::Error),
    ProtoDecode(DecodeError),
    NoTilesFound,
    NoMatchingRepository
}

impl From<bigtable_rs::bigtable::Error> for TileError {
    fn from(value: Error) -> Self {
        Self::BigTableError(value)
    }
}