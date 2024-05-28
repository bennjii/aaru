//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use rstar::{Point};
use crate::coord::latlng::LatLng;
use crate::osm;
use crate::osm::{DenseNodes, PrimitiveBlock};

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct Node {
    id: i64,
    pub position: LatLng
}

impl Point for Node {
    type Scalar = i64;
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
            1 => self.position.lng,
            _ => unreachable!(),
        }
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.position.lng,
            1 => &mut self.position.lng,
            _ => unreachable!(),
        }
    }
}

impl Node {
    /// Constructs a `Node` from a given `LatLng` and `id`.
    fn new(position: LatLng, id: &i64) -> Self {
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
        value.lon.iter()
            .zip(value.lat.iter())
            .zip(value.id.iter())
            .fold(vec![], |mut a: Vec<Self>, (coord, id)| {
                let new_node = match &a.last() {
                    Some(prior_node) => {
                        Node::new(LatLng::from(coord).delta(prior_node.position), &(id + prior_node.id))
                    },
                    None => Node::new(LatLng::from(coord), id)
                };

                a.push(new_node);
                a
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

