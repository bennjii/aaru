use std::path::PathBuf;
use std::sync::Arc;
use log::info;
use crate::codec::element::item::Element;
use crate::codec::element::iterator::ElementIterator;

#[test]
fn try_into_iter() {
    let path = PathBuf::from(crate::codec::test::DISTRICT_OF_COLUMBIA);
    let mut iter = ElementIterator::new(path).expect("Could not create iterator");

    iter.for_each(|item| {
        info!("Element: {}", item.str_type());
    });
}

#[test_log::test]
fn iter_count() {
    let path = PathBuf::from(crate::codec::test::DISTRICT_OF_COLUMBIA);
    let mut iter = ElementIterator::new(path).expect("Could not create iterator");

    let nodes = iter.map_red(|item| {
        match item {
            Element::Way(_) => 0,
            Element::Node(_) | Element::DenseNode(_) => 1,
            Element::Relation(_) => 0,
            Element::DenseNodes(_) => 0
        }
    }, |a, b| a + b, || 0);

    info!("There are {nodes} nodes");
}