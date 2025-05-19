use geohash::GeohashError;

#[derive(Debug)]
pub enum GeoError {
    // TODO: Make this &'static str
    InvalidCoordinate(String),
    GeoHashError(GeohashError),
}
