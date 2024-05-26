use crate::osm::{PrimitiveBlock};

/// `LatLng`
/// The latitude, longitude pair structure, geotags an item with a location.
///
/// ```rust
/// use aaru::coord::latlng::LatLng;
/// let latlng = LatLng::new(10, 10);
/// println!("Position: {}", latlng);
/// ```
#[derive(Debug, Clone, Copy, Hash, PartialOrd, PartialEq)]
pub struct LatLng {
    pub lng: i64,
    pub lat: i64,
}

impl From<(&i64, &i64)> for LatLng {
    /// Format is: (Lat, Lng)
    fn from(value: (&i64, &i64)) -> Self {
        LatLng {
            lat: value.0.clone(),
            lng: value.1.clone(),
        }
    }
}

impl LatLng {
    /// Constructs a new `LatLng` from a given `lat` and `lng`.
    pub fn new(lat: i64, lng: i64) -> Self {
        LatLng { lat, lng }
    }

    /// Offsets the `LatLng` from the given parent primitive.
    /// According to: https://arc.net/l/quote/ccrekhxu
    pub fn offset(mut self, group: &PrimitiveBlock) -> Self {
        self.lat = (1e-9f64 * (group.lon_offset.unwrap_or(0) as f64
            + group.granularity.unwrap_or(1) as f64 * self.lat as f64)) as i64;
        self.lng = (1e-9f64 * (group.lat_offset.unwrap_or(0) as f64
            + group.granularity.unwrap_or(1) as f64 * self.lng as f64)) as i64;
        self
    }
}