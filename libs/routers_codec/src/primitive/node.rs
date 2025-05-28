use crate::Entry;
use geo::{Destination, Distance, Euclidean, Geodesic, Point};
use rstar::{AABB, Envelope};
use std::fmt::Debug;

// TODO: Document
#[derive(Debug, Copy, Clone, PartialEq)]
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

    // TODO: Document
    pub fn bounding(&self, distance: f64) -> AABB<Point> {
        let bottom_right = Geodesic.destination(self.position, 135.0, distance);
        let top_left = Geodesic.destination(self.position, 315.0, distance);
        AABB::from_corners(top_left, bottom_right)
    }
}

impl<E> rstar::PointDistance for Node<E>
where
    E: Entry,
{
    fn distance_2(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
    ) -> <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar {
        Euclidean.distance(self.position, *point).powi(2)
    }
}

impl<E> rstar::RTreeObject for Node<E>
where
    E: Entry,
{
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.position)
    }
}
