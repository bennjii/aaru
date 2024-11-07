//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use geo::{point, Distance, Haversine, Point};
use rstar::{Envelope, AABB};
use std::ops::{Add, Mul};

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
        Haversine::distance(self.position, *point)
    }

    fn distance_2_if_less_or_equal(
        &self,
        point: &<Self::Envelope as Envelope>::Point,
        max_distance_2: <<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar,
    ) -> Option<<<Self::Envelope as Envelope>::Point as rstar::Point>::Scalar> {
        // This should utilize Envelope optimisation
        let distance = Haversine::distance(self.position, *point);
        match distance < max_distance_2 {
            true => Some(distance),
            false => None,
        }
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

    // #[inline]
    // fn decode_and_scale_diff(diffs: &[i64], scale: f64) -> Vec<f64> {
    //     const LANES: usize = Simd::<i64, 4>::LEN; // Adjust based on architecture
    //     assert_eq!(diffs.len() % LANES, 0);
    //
    //     let mut cumulative_sum = Simd::splat(0i64);
    //     let mut results = Vec::with_capacity(diffs.len());
    //
    //     for diff_chunk in diffs.chunks_exact(LANES) {
    //         let mut current = Simd::from_slice(diff_chunk);
    //
    //         // Compute the prefix sum within the SIMD register
    //         for i in 1..LANES {
    //             current[i] += current[i - 1];
    //         }
    //
    //         // Add cumulative sum from previous chunk
    //         current += Simd::splat(cumulative_sum);
    //
    //         // Update cumulative sum for the next chunk
    //         cumulative_sum = current[LANES - 1];
    //
    //         // Convert to f64 and apply scaling factor
    //         let scaled = current.cast::<f64>() * Simd::splat(scale);
    //
    //         // Store the results
    //         results.extend_from_slice(scaled.as_array());
    //     }
    //
    //     results
    // }

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
    pub fn from_dense(value: &DenseNodes, granularity: i32) -> impl Iterator<Item = Self> + '_ {
        // Nodes are at a granularity relative to `Nanodegree`
        let scaling_factor: f64 = (granularity as f64) * 1e-9f64;

        // let lon = Node::decode_and_scale_diff(value.lon.as_slice(), scaling_factor);
        // let lat = Node::decode_and_scale_diff(value.lat.as_slice(), scaling_factor);
        //
        // lon.into_iter()
        //     .zip(lat.into_iter())
        //     .zip(value.id.iter())
        //     .map(|((lon, lat), id)| {
        //         Node { position: point! { x: lon, y: lat }, id: *id }
        //     })

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
