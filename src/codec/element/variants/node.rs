//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use log::info;
use rstar::{Point};
use crate::coord::latlng::{LatLng, NanoDegree};
use crate::osm;
use crate::osm::{DenseNodes, PrimitiveBlock};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct Node {
    pub id: i64,
    pub position: LatLng
}

impl Point for Node {
    type Scalar = NanoDegree;
    const DIMENSIONS: usize = 2;

    fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
        Node {
            id: 0,
            position: LatLng::new(generator(1), generator(0))
        }
    }

    fn nth(&self, index: usize) -> Self::Scalar {
        match index {
            0 => self.position.lng,
            1 => self.position.lat,
            _ => unreachable!(),
        }
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.position.lng,
            1 => &mut self.position.lat,
            _ => unreachable!(),
        }
    }
}

impl Node {
    /// Constructs a `Node` from a given `LatLng` and `id`.
    pub(crate) fn new(position: LatLng, id: &i64) -> Self {
        Node {
            position,
            id: id.clone()
        }
    }

    /// Returns the identifier for the node
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Takes an `osm::DenseNodes` structure and extracts `Node`s as an
    /// iterator from `DenseNodes` with their contextual `PrimitiveBlock`.
    ///
    /// ```rust
    ///  use aaru::element::{item::Element, variants::Node};
    ///  use aaru::osm::PrimitiveBlock;
    ///
    /// let block: PrimitiveBlock = unimplemented!();
    ///  if let Element::DenseNodes(nodes) = block {
    ///     let nodes = Node::from_dense(nodes, &block);
    ///     for node in nodes {
    ///         println!("Node: {}", node);
    ///     }
    ///  }
    /// ```
    pub fn from_dense<'a>(value: &'a DenseNodes, block: &'a PrimitiveBlock) -> impl Iterator<Item=Self> + 'a {
        value.lat.iter()
            .zip(value.lon.iter())
            .zip(value.id.iter())
            .fold(vec![], |mut curr: Vec<Self>, ((lat, lng), id)| {
                let new_node = match &curr.last() {
                    Some(prior_node) => {
                        Node::new(
                            LatLng::delta(lat, lng, prior_node.position),
                            &(id + prior_node.id)
                        )
                    },
                    None => Node::new(LatLng::from((lat, lng)), id)
                };

                if new_node.position.lat.abs() < 5 || new_node.id == 1511122299 {
                    info!("Got interesting node...");
                    let mut k = 0;
                    k += 1;
                }

                curr.push(new_node);
                curr
            })
            .into_iter()
            // .map(|(coord, id)| Node::new(LatLng::from(coord).offset(block), id))
    }
}

impl From<&osm::Node> for Node {
    fn from(value: &osm::Node) -> Self {
        Node {
            id: value.id,
            position: LatLng::new(value.lat, value.lon)
        }
    }
}

