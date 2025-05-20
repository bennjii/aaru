use crate::proto::Tile;

pub struct MVTTile(pub(crate) Tile);

impl Into<Tile> for MVTTile {
    fn into(self) -> Tile {
        self.0
    }
}
