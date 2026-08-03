use crate::traits::Entry;

use core::cmp::{Ord, Ordering};
use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use geo::{Destination, Geodesic, Point, Rect};
use serde::{Deserialize, Serialize};

/// The standardised node primitive containing a generic
/// identifier which must implement [Entry], and contain
/// some given [Point].
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Node<E>
where
    E: Entry,
{
    pub id: E,
    pub position: Point,
}

impl<E> Node<E>
where
    E: Entry,
{
    /// Constructs a `Node` from a given `LatLng` and `id`.
    pub fn new(position: Point, id: E) -> Self {
        Self { id, position }
    }

    /// Constructs the rectangular bounding box (`[Rect](geo::Rect)`)
    /// for the square [distance](#param.distance) around the node position.
    pub fn bounding(&self, distance: f64) -> Rect<f64> {
        let bottom_right = Geodesic.destination(self.position, 135.0, distance);
        let top_left = Geodesic.destination(self.position, 315.0, distance);
        Rect::new(top_left, bottom_right)
    }
}

impl<E: Entry> Entry for Node<E> {
    fn identifier(&self) -> i64 {
        self.id.identifier()
    }
}

impl<E: Entry> Deref for Node<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<E: Entry> Default for Node<E> {
    fn default() -> Node<E> {
        Node {
            id: E::default(),
            position: Point::new(0., 0.),
        }
    }
}

impl<E: Entry> Ord for Node<E> {
    fn cmp(&self, other: &Node<E>) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<E: Entry> PartialOrd for Node<E> {
    fn partial_cmp(&self, other: &Node<E>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Entry> PartialEq for Node<E> {
    fn eq(&self, other: &Node<E>) -> bool {
        self.id.eq(&other.id)
    }
}

impl<E: Entry> Eq for Node<E> {}

impl<E: Entry> Hash for Node<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}
