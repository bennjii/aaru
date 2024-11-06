//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use geo::{point, HaversineDistance, Point};
use rstar::{Envelope, AABB};
use std::ops::{Add, Mul};
use std::simd::Simd;

use crate::codec::osm;
use crate::codec::osm::DenseNodes;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Node {
    pub id: i64,
    pub position: Point, // Coord<NanoDegree>
}

impl rstar::PointDistance for Node {
    fn distance_2(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
    ) -> <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar {
        self.position.haversine_distance(&point)
    }

    fn distance_2_if_less_or_equal(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
        max_distance_2: <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar,
    ) -> Option<<<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar> {
        // This should utilize Envelope optimisation
        let distance_2 = self.position.haversine_distance(&point);
        if distance_2 <= max_distance_2 {
            return Some(distance_2);
        }

        None
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
    #[inline]
    pub fn from_dense<'a>(
        value: &'a DenseNodes,
        granularity: i32,
    ) -> impl Iterator<Item = Self> + 'a {
        // Nodes are at a granularity relative to `Nanodegree`
        let scaling_factor: f64 = (granularity as f64) * 1e-9f64;

        let lanes = Simd::<i64, 4>::LANES; // Assuming 4 lanes for i64, but may vary by architecture
        assert!(diffs.len() == out.len());
        assert!(diffs.len() % lanes == 0);

        let mut cumulative_sum = Simd::splat(0); // Start cumulative sum as a zeroed SIMD register

        for (diffs_chunk, out_chunk) in diffs.chunks_exact(lanes).zip(out.chunks_exact_mut(lanes)) {
            // Load the current chunk into a SIMD register
            let mut current = Simd::from_slice(diffs_chunk);

            // Compute the prefix sum within the register
            for i in 1..lanes {
                current = current + current.rotate_lanes_right(i);
            }

            // Add the cumulative sum from the last chunk
            current += cumulative_sum;

            // Store the result
            current.write_to_slice(out_chunk);

            // Update cumulative sum for the next chunk (last element in this SIMD register)
            cumulative_sum = Simd::splat(current[lanes - 1]);
        }

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
                        *id + prior_node.id,
                    ),
                    None => Node::new(point! { x: lng, y: lat }.mul(scaling_factor), *id),
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
            id: value.id,
            position: point! { x: value.lon as f64, y: value.lat as f64 },
        }
    }
}
