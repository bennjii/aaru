//! We want to determine fragments as x/y/z
//! variations of the global osm set, to
//! be able to load and display generic information
//! at any level - i.e. loading raster tiles
//!
//! This is a generic implementation of a
//! slippy tile.

use fast_hilbert::xy2h;

#[derive(Debug, Clone)]
pub struct Fragment {
    x: u32,
    y: u32,
    zoom: u8
}

impl Fragment {
    pub fn new(z: &u8, x: &u32, y: &u32) -> Self {
        Self { zoom: *z, x: *x, y: *y }
    }

    pub fn with_zoom(self, z: u8) -> Self {
        Self {
            zoom: z,
            ..self
        }
    }

    /// `detail(zoom)`
    ///
    /// We want to determine the tiles visible
    /// at a specific zoom level,
    pub(crate) fn detail(self, z: u8) -> Vec<Fragment> {
        let mut target_tiles = vec![];
        let mut tiles_to_pop = vec![self];

        while let Some(tile) = tiles_to_pop.pop() {
            if tile.zoom == z {
                target_tiles.push(tile)
            } else if let Some(tile) = tile.segment() {
                tiles_to_pop.extend(tile);
            }
        }

        target_tiles
    }

    /// `segment()`
    ///
    /// Will segment the fragment into four sub-fragments.
    /// This continues until u8::MAX.
    fn segment(&self) -> Option<[Fragment; 4]> {
        match self.zoom {
            24 => None,
            _ => {
                Some([
                    (0, 0),
                    (1, 0),
                    (0, 1),
                    (1, 1),
                ].map(|(dx, dy)| Fragment {
                    // Increase zoom level
                    zoom: self.zoom + 1,
                    // Get TL, TR, BL, BR sub-fragments
                    x: (2 * self.x) + dx,
                    y: (2 * self.y) + dy,
                }))
            }
        }
    }

    pub fn to_hilbert(&self) -> u64 {
        xy2h(self.x, self.y, self.zoom)
    }
}