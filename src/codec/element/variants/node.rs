//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use std::f64::consts::PI;
use std::ops::{Add, Mul};
use geo::{point, HaversineDistance, Point};
use rstar::{Envelope, AABB};

use crate::codec::osm;
use crate::codec::osm::DenseNodes;
use crate::geo::coord::latlng::Degree;
use crate::geo::MEAN_EARTH_RADIUS;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Node {
    pub id: i64,
    pub position: Point, // Coord<NanoDegree>
}

pub struct Distance {
    meter_value: f64,
}

impl Distance {
    pub fn from_meters(meters: u32) -> Self {
        Distance {
            meter_value: meters as f64,
        }
    }

    pub fn degree(meter_value: f64) -> Degree {
        let delta_lat = (meter_value / MEAN_EARTH_RADIUS) * (180f64 / PI);
        let delta_lng = (meter_value / MEAN_EARTH_RADIUS * (PI / 4f64).cos()) * (180f64 / PI);
        let avg_delta = (delta_lat + delta_lng) / 2f64;
        avg_delta / 1f64
    }
}

impl rstar::PointDistance for Node {
    fn distance_2(&self, point: &<Self::Envelope as Envelope>::Point)
        -> <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar
    {
        self.position.haversine_distance(&point)
    }
}

impl rstar::RTreeObject for Node {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.position)
    }
}


// TODO: Evaluate the necessity of Node's contents
// impl rstar::Point for Node {
//     type Scalar = Degree;
//     const DIMENSIONS: usize = 2;
//
//     #[inline]
//     fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
//         // Going to happen for EVERY item
//         Node {
//             id: 0,
//             position: point! { x: generator(0), y: generator(1) },
//         }
//     }
//
//     #[inline]
//     fn nth(&self, index: usize) -> Self::Scalar {
//         match index {
//             0 => self.position.x(),
//             1 => self.position.y(),
//             _ => unreachable!(),
//         }
//     }
//
//     #[inline]
//     fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
//         match index {
//             0 => self.position.x_mut(),
//             1 => self.position.y_mut(),
//             _ => unreachable!(),
//         }
//     }
// }

impl Node {
    /// Constructs a `Node` from a given `LatLng` and `id`.
    pub(crate) fn new(position: Point, id: i64) -> Self {
        Node { position, id }
    }

    /// Returns the identifier for the node
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Takes an `osm::DenseNodes` structure and extracts `Node`s as an
    /// iterator from `DenseNodes` with their contextual `PrimitiveBlock`.
    ///
    /// ```rust,ignore
    ///  use aaru::codec::element::{item::Element, variants::Node};
    ///  use aaru::codec::osm::PrimitiveBlock;
    ///
    /// let block: PrimitiveBlock = unimplemented!();
    ///  if let Element::DenseNodes(nodes) = block {
    ///     let nodes = Node::from_dense(nodes);
    ///     for node in nodes {
    ///         println!("Node: {}", node);
    ///     }
    ///  }
    /// ```
    pub fn from_dense<'a>(value: &'a DenseNodes, granularity: i32) -> impl Iterator<Item = Self> + 'a {
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
                        prior_node.position.add(point! { x: lng, y: lat }.mul(scaling_factor)),
                        *id + prior_node.id,
                    ),
                    None => Node::new(point! { x: lng, y: lat }.mul(scaling_factor), *id),
                };

                curr.push(new_node);
                curr
            })
            .into_iter()
    }

    pub fn to(&self, other: &Node) -> Distance {
        let lat: f64 = self.position.y() - other.position.y();
        let lng: f64 = self.position.x() - other.position.x();

        let a = (lng / 2.0f64).sin().powi(2)
            + self.position.y().cos() * other.position.y().cos() * (lat / 2.0f64).sin().powi(2);
        let c = 2f64 * a.sqrt().asin();
        Distance::from_meters((6371008.8 * c) as u32)
    }
}

impl From<&osm::Node> for Node {
    fn from(value: &osm::Node) -> Self {
        Node {
            id: value.id,
            position: point! { x: value.lon as f64, y: value.lat as f64 },
        }
    }
}
