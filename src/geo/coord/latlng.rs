use std::fmt::{Debug, Formatter};
use crate::osm::{PrimitiveBlock};

/// `LatLng`
/// The latitude, longitude pair structure, geotags an item with a location.
///
/// ```rust
/// use aaru::coord::latlng::LatLng;
/// let latlng = LatLng::new(10, 10);
/// println!("Position: {}", latlng);
/// ```
#[derive(Clone, Copy, PartialOrd, PartialEq)]
pub struct LatLng {
    pub lng: f64,
    pub lat: f64,
}

impl From<(&i64, &i64)> for LatLng {
    /// Format is: (Lat, Lng)
    fn from(value: (&i64, &i64)) -> Self {
        LatLng {
            lat: value.0.clone() as f64,
            lng: value.1.clone() as f64,
        }
    }
}

impl From<(&f64, &f64)> for LatLng {
    /// Format is: (Lat, Lng)
    fn from(value: (&f64, &f64)) -> Self {
        LatLng::new_raw(value.0.clone(), value.1.clone())
    }
}

impl LatLng {
    /// Constructs a new `LatLng` from a given `lat` and `lng`.
    pub fn new_7(lat: i64, lng: i64) -> Self {
        LatLng {
            lat: lat as f64 * 1e-7f64,
            lng: lng as f64 * 1e-7f64
        }
    }

    pub fn new_raw(lat: f64, lng: f64) -> Self {
        LatLng { lat, lng }
    }

    pub fn lat(&self) -> f64 {
        self.lat
    }

    pub fn lng(&self) -> f64 {
        self.lng
    }

    /// Offsets the `LatLng` from the given parent primitive.
    /// According to: https://arc.net/l/quote/ccrekhxu
    pub fn offset(&mut self, group: &PrimitiveBlock) -> &mut Self {
        let granularity = group.granularity.unwrap_or(1) as f64;
        let nano_degree = 1e-9f64;

        self.lat = nano_degree * (group.lon_offset.unwrap_or(0) as f64 + (granularity * self.lat));
        self.lng = nano_degree * (group.lat_offset.unwrap_or(0) as f64 + (granularity * self.lng));

        self
    }

    pub fn delta(mut self, prior: Self) -> Self {
        // Delta encoding (difference only)
        self.lat += prior.lat;
        self.lng += prior.lng;

        self
    }
}

impl Debug for LatLng {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "POINT({} {})", self.lat as f64 * 1e-7f64, self.lng as f64 * 1e-7f64)
    }
}