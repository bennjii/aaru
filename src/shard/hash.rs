use std::hash::{Hash, Hasher};
use geo::Coord;
use crate::auto;

impl<'a> Hash for auto::WaySide {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let coords: Coord<i64> = self.nodes
            .iter()
            .fold(Coord::zero(), |old_coord, coord|
                Coord::from((old_coord.x + coord.x, old_coord.y + coord.y))
            );

        let (lat, lng) = coords.x_y();
        state.write_i64(lat / self.nodes.len() as i64);
        state.write_i64(lng / self.nodes.len() as i64);

        state.finish();
    }

    fn hash_slice<H: Hasher>(_: &[Self], _: &mut H) where Self: Sized {
        todo!()
    }
}

impl<'a, T: Hash> Hash for auto::ShardData<T> {
    fn hash<H: Hasher>(&self, _: &mut H) {
        // self.nodes.type_id();
        // self.nodes.into_iter(&Default::default()).map(|node| node.hash(state)))
    }

    fn hash_slice<H: Hasher>(_: &[Self], _: &mut H) where Self: Sized {
        todo!()
    }
}