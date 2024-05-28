//! The `Element` item, provides distinction for
//! Nodes, DenseNodes, ... by reference to their
//! derived item, in the primitive entity.

use std::vec;
use crate::osm;
use crate::element::variants::{Node, Way};
use crate::osm::{PrimitiveBlock, PrimitiveGroup};

#[derive(Clone)]
pub enum Element<'a> {
    Node(&'a osm::Node),
    Way(&'a osm::Way),
    DenseNodes(&'a osm::DenseNodes),
    Relation(&'a osm::Relation)
}

#[derive(Clone)]
pub enum ProcessedElement {
    Node(Node),
    Way(Way)
}

impl ProcessedElement {
    pub(crate) fn from_raw(element: Element, block: &PrimitiveBlock) -> Vec<ProcessedElement>{
        match element {
            Element::DenseNodes(dense_nodes) => {
                Node::from_dense(dense_nodes, block)
                    .map(|node| ProcessedElement::Node(node))
                    .collect()
            },
            Element::Node(node) =>
                vec![ProcessedElement::Node(Node::from(node))],
            Element::Way(way) =>
                vec![ProcessedElement::Way(Way::from_raw(way, block))],
            _ => vec![]
        }
    }
}

impl<'a> Element<'a> {
    #[inline]
    pub(crate) fn from_group(group: &'a PrimitiveGroup) -> Vec<Element<'a>> {
        let mut elements: Vec<Element<'a>> = Vec::new();

        elements.extend(group.ways.iter().map(|way| Element::Way(way)));
        elements.extend(group.nodes.iter().map(|node| Element::Node(node.into())));
        elements.extend(group.relations.iter().map(|relation| Element::Relation(relation)));

        if let Some(nodes) = &group.dense {
            elements.push(Element::DenseNodes(nodes));
        }

        elements
    }

    pub(crate) fn str_type(&self) -> &str {
        match self {
            Element::Node(_) => "node",
            Element::Way(_) => "way",
            Element::Relation(_) => "relation",
            Element::DenseNodes(_) => "node set",
        }
    }
}