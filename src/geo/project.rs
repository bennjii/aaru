//! Required structures to project between standards

use crate::geo::coord::point::Point;
use crate::tile::layer::MVT_EXTENT;

/// Allows for projection between two standards.
pub trait Project {
    /// Projects a position between two standards.
    /// It Takes an input and a zoom level, and outputs a value implementing Point.
    ///
    /// ### Example
    /// ```rust,no_run
    /// use aaru::geo::{LatLng, Project};
    /// use aaru::geo::project::SlippyTile;
    ///
    /// let value = LatLng::from_degree_unchecked(38.9126, -77.0234);
    /// let SlippyTile((x, px), (y, py), z) = SlippyTile::project(&value, 19);
    /// // We now have the slippy tile coordinate of the original lat/lng.
    /// ```
    fn project<G, T, const N: usize>(value: &T, zoom: u8) -> Self where T: Point<G, N>;
}

#[doc(hidden)]
pub mod projections {
    use crate::geo::coord::latlng::LatLng;

    /// In the definition given here, it simply wraps a LatLng definition of a point.
    /// *Learn more [here](https://en.wikipedia.org/wiki/Web_Mercator_projection?useskin=vector).*
    pub struct WebMercator(pub LatLng);

    /// A Slippy tile is one which has a defined x and y, which is distinct to its zoom level.
    /// It can also have a distance *inside* the tile, which is the 2nd parameter.
    /// On a tile corner, this delta value is 0.
    ///
    /// To discover this for yourself, use an explorer tool like [this one](https://chrishewett.com/blog/slippy-tile-explorer/).
    ///
    /// ```rust,no_run
    /// use aaru::geo::project::SlippyTile;
    /// // Tile which encloses central europe
    /// let value = SlippyTile((4, 0), (8, 0), 5);
    /// ```
    /// *Learn more [at the osm wiki](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames).*
    pub struct SlippyTile(pub (u32, u32), pub (u32, u32), pub u8);
}

#[doc(inline)]
pub use projections::SlippyTile;
#[doc(inline)]
pub use projections::WebMercator;

impl Project for SlippyTile {
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

        SlippyTile((xn, xoff), (yn, yoff), zoom)
    }
}

impl Project for WebMercator {
    fn project<G, T, const N: usize>(value: &T, _: u8) -> Self
    where
        T: Point<G, N>,
    {
        WebMercator(value.lat_lng())
    }
}