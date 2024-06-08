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

pub struct Distance {
    meter_value: u32
}

impl Distance {
    pub fn from_meters(meters: u32) -> Self {
        Distance {
            meter_value: meters
        }
    }

    pub fn as_km(&self) -> f32 {
        (self.meter_value as f32) / 1000f32
    }

    pub fn as_m(&self) -> u32 {
        self.meter_value
    }
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

                curr.push(new_node);
                curr
            })
            .into_iter()
            // .map(|(coord, id)| Node::new(LatLng::from(coord).offset(block), id))
    }

    pub fn to(&self, other: &Node) -> Distance {
        let lat: f64 = (self.position.lat() - other.position.lat()) * 1e-2;
        let lng: f64 = (self.position.lng() - other.position.lng()) * 1e-2;

        let a = (lng / 2.0f64).sin().powi(2) + self.position.lat().cos() * other.position.lat().cos() * (lat / 2.0f64).sin().powi(2);
        let c = 2f64 * a.sqrt().asin();
        Distance::from_meters((6371008.8 * c) as u32)
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

