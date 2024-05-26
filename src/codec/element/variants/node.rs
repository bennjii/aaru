//! Describes a simplification of an `osm::Node`. Stripping it
//! of the context information required for changelogs, and utilising
//! only the elements required for graph routing.

use crate::coord::latlng::LatLng;
use crate::osm;
use crate::osm::{DenseNodes, PrimitiveBlock};

#[derive(Copy, Clone)]
pub struct Node {
    id: i64,
    position: LatLng
}

impl Node {
    /// Constructs a `Node` from a given `LatLng` and `id`.
    fn new(position: LatLng, id: &i64) -> Self {
        Node {
            position,
            id: id.clone()
        }
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
            .map(|(coord, id)| Node::new(LatLng::from(coord).offset(block), id))
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

