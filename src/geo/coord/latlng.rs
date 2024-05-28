use std::fmt::{Debug, Formatter};
use crate::osm::{PrimitiveBlock};

pub type NanoDegree = i64;
pub type Degree = f64;

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
    pub lng: NanoDegree,
    pub lat: NanoDegree,
}

impl From<(&i64, &i64)> for LatLng {
    /// Format is: (Lat, Lng)
    fn from((lat, lng): (&i64, &i64)) -> Self {
        Self::new(lat.clone(), lng.clone())
    }
}

impl LatLng {
    /// Constructs a new `LatLng` from a given `lat` and `lng`.
    pub fn new(lat: NanoDegree, lng: NanoDegree) -> Self {
        LatLng { lat, lng }
    }

    pub fn from_degree(lat: Degree, lng: Degree) -> Self {
        assert!(lat > -90f64 && lat < 90f64);
        assert!(lng < 180f64 && lng > -180f64);

        LatLng {
            lat: (lat * 1e7) as i64,
            lng: (lng * 1e7) as i64
        }
    }

    pub fn lat(&self) -> Degree {
        self.lat as f64 * 1e-7
    }

    pub fn nano_lat(&self) -> NanoDegree {
        self.lat
    }

    pub fn lng(&self) -> Degree {
        self.lng as f64 * 1e-7
    }

    pub fn nano_lng(&self) -> NanoDegree {
        self.lng as i64
    }

    /// Offsets the `LatLng` from the given parent primitive.
    /// According to: https://arc.net/l/quote/ccrekhxu
    pub fn offset(&mut self, group: &PrimitiveBlock) -> &mut Self {
        let granularity = group.granularity.unwrap_or(1) as NanoDegree;

        self.lat = group.lon_offset.unwrap_or(0) + (granularity * self.lat);
        self.lng = group.lat_offset.unwrap_or(0) + (granularity * self.lng);

        self
    }

    // Delta encoding (difference only)
    pub fn delta(lat: &i64, lng: &i64, prior: Self) -> Self {
        LatLng {
            lat: lat + prior.nano_lat(),
            lng: lng + prior.nano_lng()
        }
    }
}

impl Debug for LatLng {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "POINT({} {})", self.lng(), self.lat())
    }
}