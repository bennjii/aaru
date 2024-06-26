use crate::geo::coord::latlng::LatLng;

pub trait Point<T, const N: usize> {
    /// `id()`
    ///
    /// Returns the identifier of the location pointed to.
    /// This has different representative meaning, according
    /// to where it is located, and what it represents.
    fn id(&self) -> u64;

    /// `lat_lng()`
    ///
    /// Returns a `LatLng` pair pointing to the origin of
    /// the data the structure represents.
    fn lat_lng(&self) -> LatLng;

    /// `keys()`
    fn keys() -> [String; N];

    /// `values()`
    fn values(&self) -> [T; N];
}