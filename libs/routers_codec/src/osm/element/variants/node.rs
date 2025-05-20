//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use geo::{Destination, Distance, Euclidean, Geodesic, Point, point};
use rstar::{AABB, Envelope};
use std::ops::{Add, Mul};

use super::common::OsmEntryId;
use crate::osm;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Node {
    pub id: OsmEntryId,
    pub position: Point,
}

impl rstar::PointDistance for Node {
    fn distance_2(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
    ) -> <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar {
        Euclidean.distance(self.position, *point).powi(2)
    }
}

impl rstar::RTreeObject for Node {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.position)
    }
}

impl Node {
    /// Constructs a `Node` from a given `LatLng` and `id`.
    pub const fn new(position: Point, id: OsmEntryId) -> Self {
        Node { position, id }
    }

    /// Returns the identifier for the node
    pub fn id(&self) -> OsmEntryId {
        self.id
    }

    pub fn bounding(&self, distance: f64) -> AABB<Point> {
        let bottom_right = Geodesic.destination(self.position, 135.0, distance);
        let top_left = Geodesic.destination(self.position, 315.0, distance);
        AABB::from_corners(top_left, bottom_right)
    }

    /// Takes an `osm::DenseNodes` structure and extracts `Node`s as an
    /// iterator from `DenseNodes` with their contextual `PrimitiveBlock`.
    ///
    /// ```rust
    ///  use routers_codec::osm::element::{item::Element, variants::Node};
    ///  use routers_codec::osm::PrimitiveBlock;
    ///
    /// let block: PrimitiveBlock = unimplemented!();
    ///  if let Element::DenseNodes(nodes) = block {
    ///     let nodes = Node::from_dense(nodes, 100);
    ///     for node in nodes {
    ///         println!("Node: {}", node);
    ///     }
    ///  }
    /// ```
    #[inline]
    pub fn from_dense(
        value: &osm::DenseNodes,
        granularity: i32,
    ) -> impl Iterator<Item = Self> + '_ {
        // Nodes are at a granularity relative to `Nanodegree`
        let scaling_factor: f64 = (granularity as f64) * 1e-9f64;

        value
            .lon
            .iter()
            .map(|v| *v as f64)
            .zip(value.lat.iter().map(|v| *v as f64))
            .zip(value.id.iter())
            .fold(vec![], |mut curr: Vec<Self>, ((lng, lat), id)| {
                let new_node = match &curr.last() {
                    Some(prior_node) => Node::new(
                        prior_node
                            .position
                            .add(point! { x: lng, y: lat }.mul(scaling_factor)),
                        prior_node.id + *id,
                    ),
                    None => Node::new(
                        point! { x: lng, y: lat }.mul(scaling_factor),
                        OsmEntryId::from(*id),
                    ),
                };

                curr.push(new_node);
                curr
            })
            .into_iter()
    }
}

impl From<&osm::Node> for Node {
    fn from(value: &osm::Node) -> Self {
        Node {
            id: OsmEntryId::node(value.id),
            position: point! { x: value.lon as f64, y: value.lat as f64 },
        }
    }
}
