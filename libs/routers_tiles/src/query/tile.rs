use crate::proto::Tile;

pub struct MVTTile(pub(crate) Tile);

impl From<MVTTile> for Tile {
    fn from(val: MVTTile) -> Self {
        val.0
    }
}
