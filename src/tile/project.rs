use crate::coord::latlng::LatLng;
use crate::coord::point::Point;
use crate::tile::layer::MVT_EXTENT;

pub trait Project {
    fn project<G, T>(value: T) -> (LatLng, LatLng) where T: Point<G>;
}

pub mod projections {
    pub struct WebMercator;
}

impl Project for projections::WebMercator {
    // TODO: Refactor the return type to latlng/offset/... when known
    fn project<G, T>(value: T) -> (LatLng, u32, u32)
    where
        T: Point<G>,
    {
        let offset = |value: f64| {
            let n = value.floor() as u32;
            let offset = ((MVT_EXTENT as f64) * (value - value.floor())) as u32;
            (n, offset)
        };

        let zoom = T::ZOOM;
        let (lat, lng) = value.lat_lng().expand();

        let shl_zoom = 1 << zoom;
        let x = shl_zoom as f64 * ((lng + 180.0) / 360.0);
        let (xn, xoff) = offset(x);

        let y1 = (lat * 0.0174533).tan();
        let y2 = 1.0 / (lat * 0.0174533).cos();
        let y = shl_zoom as f64 * (1.0 - (y1 + y2).ln() / std::f64::consts::PI) / 2.0;
        let (yn, yoff) = offset(y);

        (LatLng { lng: xn as i64, lat: yn as i64 }, xoff, yoff)
    }
}