use crate::geo::coord::point::Point;
use crate::tile::layer::MVT_EXTENT;

pub trait Project {
    fn project<G, T, const N: usize>(value: &T, zoom: u8) -> Self where T: Point<G, N>;
}

pub mod projections {
    use crate::geo::coord::latlng::LatLng;

    pub struct WebMercator(pub LatLng);
    pub struct SlippyTile(pub (u32, u32), pub (u32, u32));
}

impl Project for projections::SlippyTile {
    /// See the [OSM Wiki](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames#Mathematics) for the projection source.
    fn project<G, T, const N: usize>(value: &T, zoom: u8) -> Self
    where
        T: Point<G, N>,
    {
        let offset = |value: f64| {
            let n = value.floor() as u32;
            let offset = ((MVT_EXTENT as f64) * (value - value.floor())) as u32;
            (n, offset)
        };

        // Get the Lat/Lng for the values origin
        let (lat, lng) = value.lat_lng().expand();

        // Obtain the X tile position (at desired zoom) and offset inside tile
        let shl_zoom = 1 << zoom;
        let x = shl_zoom as f64 * ((lng + 180.0) / 360.0);
        let (xn, xoff) = offset(x);

        // Same for Y tile position
        let y1 = (lat * 0.0174533).tan();
        let y2 = 1.0 / (lat * 0.0174533).cos();
        let y = shl_zoom as f64 * (1.0 - (y1 + y2).ln() / std::f64::consts::PI) / 2.0;
        let (yn, yoff) = offset(y);

        projections::SlippyTile((xn, xoff), (yn, yoff))
    }
}

impl Project for projections::WebMercator {
    fn project<G, T, const N: usize>(value: &T, _: u8) -> Self
    where
        T: Point<G, N>,
    {
        projections::WebMercator(value.lat_lng())
    }
}