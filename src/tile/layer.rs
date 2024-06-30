use std::marker::PhantomData;

use crate::geo::coord::point::Point;
use crate::codec::mvt::{Feature, GeomType, Layer, Value};
use crate::geo::{MVT_EXTENT, MVT_VERSION};
use crate::geo::project::Project;
use crate::geo::project::projections::SlippyTile;

struct TileLayer<const N: usize> {
    layer: Layer,
    breath: PhantomData<[(); N]>
}

impl<const N: usize> TileLayer<N> {
    fn new(layer: Layer) -> Self {
        Self { layer, breath: PhantomData::default() }
    }
}

pub struct MVTFeature<const N: usize>(pub Feature);
pub struct MVTLayer<const N: usize>(pub Layer);

impl<T, const N: usize> From<(Vec<T>, u8, String)> for MVTLayer<N>
    where T: Point<Value, N>
{
    fn from((value, zoom, name): (Vec<T>, u8, String)) -> Self {
        let keys = T::keys();
        let values = value.iter().flat_map(|v| v.values()).collect();

        let features = value
            .iter()
            .enumerate()
            .map(|(index, value)|
                MVTFeature::from((index, zoom, value)).0
            )
            .collect();

        MVTLayer(Layer {
            // TODO: Implement-Me properly
            name,
            values,
            features,
            extent: Some(MVT_EXTENT),
            version: MVT_VERSION,
            keys: keys.iter().map(|k| k.to_string()).collect()
        })
    }
}

impl<T, const N: usize> From<(usize, u8, &T)> for MVTFeature<N> where T: Point<Value, N> {
    fn from((index, zoom, value): (usize, u8, &T)) -> Self {
        let key_length: u32 = T::keys().len() as u32;

        // We know we're centered to the tile corner, so we just need it's
        // internal offset.
        let SlippyTile((_, px), (_, py), _) = SlippyTile::project(value, zoom);

        fn zig(value: u32) -> u32 {
            (value << 1) ^ (value >> 31)
        }

        Self(Feature {
            id: Some(value.id()),
            tags: (0..key_length)
                .into_iter()
                .flat_map(|i| [i, (index as u32) * key_length + i])
                .collect(),
            r#type: Some(i32::from(GeomType::Point)),
            geometry: vec![(1 & 0x7) | (1 << 3), zig(px), zig(py)]
        })
    }
}