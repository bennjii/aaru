use crate::osm;
use crate::osm::PrimitiveGroup;

#[derive(Copy, Clone)]
pub enum Element<'a> {
    Node(&'a osm::Node),
    DenseNode(&'a osm::DenseNodes),
    Way(&'a osm::Way),
    Relation(&'a osm::Relation)
}

impl<'a> Element<'a> {
    #[inline]
    pub(crate) fn from_group(group: &'a PrimitiveGroup) -> Vec<Element<'a>> {
        let mut elements: Vec<Element<'a>> = Vec::new();

        elements.extend(group.ways.iter().map(|way| Element::Way(way)));
        elements.extend(group.nodes.iter().map(|node| Element::Node(node)));
        elements.extend(group.dense.iter().map(|dense| Element::DenseNode(dense)));
        elements.extend(group.relations.iter().map(|relation| Element::Relation(relation)));

        elements
    }

    pub(crate) fn str_type(&self) -> &str {
        match self {
            Element::Node(_) | Element::DenseNode(_) => "node",
            Element::Way(_) => "way",
            Element::Relation(_) => "relation"
        }
    }
}