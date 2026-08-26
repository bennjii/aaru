//! The Overture `connector` feature — a graph node.
//!
//! A connector is a bare junction point: an id and a `Point`. Segments are
//! stitched into a graph by referencing shared connector ids.

use geo::Point;

use crate::overture::id::OvertureEntryId;

/// A junction point shared between segments; becomes a graph node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Connector {
    pub id: OvertureEntryId,
    pub position: Point,
}

impl Connector {
    #[inline]
    pub fn new(id: OvertureEntryId, position: Point) -> Self {
        Self { id, position }
    }
}
