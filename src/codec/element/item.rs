//! The `Element` item, provides distinction for
//! Nodes, DenseNodes, ... by reference to their
//! derived item, in the primitive entity.

use std::vec;
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::codec::element::variants::{Node, Way};
use crate::codec::osm;
use crate::codec::osm::{PrimitiveBlock, PrimitiveGroup};

#[derive(Clone)]
pub enum Element<'a> {
    Node(&'a osm::Node),
    Way(&'a osm::Way),
    DenseNodes(&'a osm::DenseNodes),
    Relation(&'a osm::Relation),
}

#[derive(Clone)]
pub enum ProcessedElement {
    Node(Node),
    Way(Way),
}

impl ProcessedElement {
    #[inline]
    pub(crate) fn from_raw(element: Element, block: &PrimitiveBlock) -> Vec<ProcessedElement> {
        #[cfg(feature = "tracing")]
        if block.lat_offset.is_some() || block.lon_offset.is_some() || block.granularity.is_some() {
            debug!(
                "BlockHasOffset! +Lon={:?}, +Lat={:?}, Granularity={:?}",
                block.lon_offset, block.lat_offset, block.granularity
            );
        }

        // Default Scaling Factor: https://wiki.openstreetmap.org/wiki/PBF_Format
        let granularity = block.granularity.unwrap_or(100);

        match element {
            // TODO: Try making just an iterator return
            Element::DenseNodes(dense_nodes) => Node::from_dense(dense_nodes, granularity)
                .map(ProcessedElement::Node)
                .collect(),
            Element::Node(node) => vec![ProcessedElement::Node(Node::from(node))],
            Element::Way(way) => vec![ProcessedElement::Way(Way::from_raw(way, block))],
            _ => vec![],
        }
    }
}

impl<'a> Element<'a> {
    #[inline]
    pub(crate) fn from_group(group: &'a PrimitiveGroup) -> Vec<Element<'a>> {
        let mut elements: Vec<Element<'a>> = Vec::new();

        elements.extend(group.ways.iter().map(Element::Way));
        elements.extend(group.nodes.iter().map(Element::Node));
        elements.extend(
            group
                .relations
                .iter()
                .map(Element::Relation),
        );

        if let Some(nodes) = &group.dense {
            elements.push(Element::DenseNodes(nodes));
        }

        elements
    }

    pub fn str_type(&self) -> &str {
        match self {
            Element::Node(_) => "node",
            Element::Way(_) => "way",
            Element::Relation(_) => "relation",
            Element::DenseNodes(_) => "node set",
        }
    }
}
