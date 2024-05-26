//! The `Element` item, provides distinction for
//! Nodes, DenseNodes, ... by reference to their
//! derived item, in the primitive entity.

use log::info;
use crate::coord::latlng::LatLng;
use crate::osm;
use crate::osm::{PrimitiveBlock, PrimitiveGroup};

#[derive(Copy, Clone)]
pub enum Element<'a> {
    DenseNode(LatLng),
    Node(&'a osm::Node),
    DenseNodes(&'a osm::DenseNodes),
    Way(&'a osm::Way),
    Relation(&'a osm::Relation)
}

impl<'a> Element<'a> {
    #[inline]
    pub(crate) fn from_group(group: &'a PrimitiveGroup, block: &'a PrimitiveBlock) -> Vec<Element<'a>> {
        let mut elements: Vec<Element<'a>> = Vec::new();

        info!("{} Ways, {} Nodes, {} Dense Nodes, {} Relations", group.ways.len(), group.nodes.len(), group.dense.is_some(), group.relations.len());

        elements.extend(group.ways.iter().map(|way| Element::Way(way)));
        elements.extend(group.nodes.iter().map(|node| Element::Node(node)));
        elements.extend(group.relations.iter().map(|relation| Element::Relation(relation)));

        if let Some(nodes) = &group.dense {
            elements.extend(nodes.lon.iter()
                .zip(nodes.lat.iter())
                .map(|coord| Element::DenseNode(LatLng::from(coord).offset(block))));

            elements.push(Element::DenseNodes(nodes));
        }

        elements
    }

    pub(crate) fn str_type(&self) -> &str {
        match self {
            Element::Node(_) | Element::DenseNode(_) => "node",
            Element::Way(_) => "way",
            Element::Relation(_) => "relation",
            Element::DenseNodes(_) => "node set",
        }
    }
}