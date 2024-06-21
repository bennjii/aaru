use std::marker::PhantomData;
use crate::geo::coord::point::Point;
use crate::codec::mvt::{Feature, GeomType, Layer, Value};
use crate::tile::project::Project;
use crate::tile::project::projections::WebMercator;

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

impl<T> From<Vec<T>> for Layer
    where T: Point<Value, 2>
{
    fn from(value: Vec<T>) -> Self {
        let keys = T::keys();
        let values = value.iter().flat_map(|v| v.values()).collect();

        let features = value
            .iter()
            .enumerate()
            .map(|(index, point)| {
                Feature::from(point)
                // let (lng, lat) = point.lnglat();
                // let (_, _, px, py) = slippy_mvt(zoom, lng, lat);
                // let tags = create_tags(index as u32, keys.len() as u32);
                // Feature::from_point(Some(point.id()), px, py, tags)
            })
            .collect();

        Layer {
            // TODO: Implement-Me properly
            name: "".to_string(),
            values,
            features,
            extent: Some(MVT_EXTENT),
            version: MVT_VERSION,
            keys: keys.iter().map(|k| k.to_string()).collect()
        }
    }
}

impl<T> From<&T> for Feature where T: Point<Value, 2> {
    fn from(value: &T) -> Self {
        let buffer_size: u32 = 0;
        let key_length: u32 = T::keys().len() as u32;

        let (_, px, py) = WebMercator::project(value);

        fn zig(value: u32) -> u32 {
            (value << 1) ^ (value >> 31)
        }

        Self {
            id: Some(value.id()),
            tags: (0..key_length)
                .into_iter()
                .flat_map(|i| [i, buffer_size * key_length + i])
                .collect(),
            r#type: Some(i32::from(GeomType::Point)),
            geometry: vec![(1 & 0x7) | (1 << 3), zig(px), zig(py)]
        }
    }
}