#![cfg(test)]

use crate::codec::consts::DISTRICT_OF_COLUMBIA;
use crate::codec::element::item::Element;
use crate::codec::element::item::ProcessedElement;
use crate::codec::element::iterator::ElementIterator;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::parallel::Parallel;
use log::info;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn try_into_iter() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ElementIterator::new(path).expect("Could not create iterator");

    iter.for_each(|item| {
        info!("Element: {}", item.str_type());
    });
}

#[test]
fn iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ElementIterator::new(path).expect("Could not create iterator");
    let now = Instant::now();

    let nodes = iter.map_red(
        |item| match item {
            Element::Way(_) => 0,
            Element::Node(_) => 1,
            Element::Relation(_) => 0,
            Element::DenseNodes(_) => 0,
        },
        |a, b| a + b,
        || 0,
    );

    println!("There are {nodes} nodes");
    println!("Took: {}ms", now.elapsed().as_micros() / 1000)
}

#[test]
fn processed_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ProcessedElementIterator::new(path).expect("Could not create iterator");

    let now = Instant::now();

    let nodes = iter.map_red(
        |item| match item {
            ProcessedElement::Way(_) => 0,
            ProcessedElement::Node(_) => 1,
        },
        |a, b| a + b,
        || 0,
    );

    println!("There are {nodes} nodes");
    println!("Took: {}ms", now.elapsed().as_micros() / 1000)
}
