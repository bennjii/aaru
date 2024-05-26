use crate::osm::{PrimitiveBlock};

#[derive(Debug, Clone, Copy, Hash)]
pub struct LatLng {
    lng: i64,
    lat: i64,
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
    fn new(lat: i64, lng: i64) -> Self {
        LatLng { lat, lng }
    }

    pub fn offset(mut self, group: &PrimitiveBlock) -> Self {
        self.lat += group.lat_offset.unwrap_or(0);
        self.lng += group.lon_offset.unwrap_or(0);
        self
    }
}