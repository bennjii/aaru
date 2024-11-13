use geohash::GeohashError;
use crate::impl_err;

#[derive(Debug)]
pub enum GeoError {
    // TODO: Make this &'static str
    InvalidCoordinate(String),
    GeoHashError(GeohashError)
}

impl_err!(GeohashError, GeoError, GeoHashError);