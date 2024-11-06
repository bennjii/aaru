use crate::geo::coord::latlng::LatLng;

/// A generic trait used for tiling a point
/// using the `MVT` schema.
pub trait TileItem<T, const N: usize> {
    /// Returns the identifier of the location pointed to.
    /// This has different representative meaning, according
    /// to where it is located, and what it represents.
    fn id(&self) -> u64;

    /// Returns a `LatLng` pair pointing to the origin of
    /// the data the structure represents.
    fn lat_lng(&self) -> LatLng;

    /// Outputs the sized string array
    fn keys<'a>() -> [&'a str; N];

    /// Outputs the sized `T` array of values
    fn values(&self) -> [T; N];
}

impl TileItem<(), 0> for LatLng {
    fn id(&self) -> u64 {
        todo!()
    }

    fn lat_lng(&self) -> LatLng {
        self.clone()
    }

    fn keys<'a>() -> [&'a str; 0] {
        todo!()
    }

    fn values(&self) -> [(); 0] {
        todo!()
    }
}
