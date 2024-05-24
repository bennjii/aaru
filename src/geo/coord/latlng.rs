#[derive(Debug, Clone, Copy)]
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

}