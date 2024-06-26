use std::marker::PhantomData;

use crate::geo::coord::point::Point;
use crate::codec::mvt::{Feature, GeomType, Layer, Value};
use crate::tile::project::Project;
use crate::tile::project::projections::{SlippyTile};

pub const MVT_EXTENT: u32 = 4096;
pub const MVT_VERSION: u32 = 2;

struct TileLayer<const N: usize> {
    layer: Layer,
    breath: PhantomData<[(); N]>
}

impl<const N: usize> TileLayer<N> {
    fn new(layer: Layer) -> Self {
        Self { layer, breath: PhantomData::default() }
    }
}

impl<T> From<(Vec<T>, u8)> for Layer
    where T: Point<Value, 2>
{
    fn from((value, zoom): (Vec<T>, u8)) -> Self {
        let keys = T::keys();
        let values = value.iter().flat_map(|v| v.values()).collect();

        let features = value
            .iter()
            .enumerate()
            .map(|(index, value)| Feature::from((index, zoom, value)))
            .collect();

        Layer {
            // TODO: Implement-Me properly
            name: "brakepoint_layer".to_string(),
            values,
            features,
            extent: Some(MVT_EXTENT),
            version: MVT_VERSION,
            keys: keys.iter().map(|k| k.to_string()).collect()
        }
    }
}

impl<T> From<(usize, u8, &T)> for Feature where T: Point<Value, 2> {
    fn from((index, zoom, value): (usize, u8, &T)) -> Self {
        let key_length: u32 = T::keys().len() as u32;

        let SlippyTile((_, px), (_, py)) = SlippyTile::project(value, zoom);

        fn zig(value: u32) -> u32 {
            (value << 1) ^ (value >> 31)
        }

        Self {
            id: Some(value.id()),
            tags: (0..key_length)
                .into_iter()
                .flat_map(|i| [i, (index as u32) * key_length + i])
                .collect(),
            r#type: Some(i32::from(GeomType::Point)),
            geometry: vec![(1 & 0x7) | (1 << 3), zig(px), zig(py)]
        }
    }
}