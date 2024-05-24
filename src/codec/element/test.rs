use std::path::PathBuf;
use log::info;
use crate::codec::element::iterator::ElementIterator;

#[test_log::test]
fn try_into_iter() {
    let path = PathBuf::from(crate::codec::test::DISTRICT_OF_COLUMBIA);
    let mut iter = ElementIterator::new(path).expect("Could not create iterator");

    iter.for_each(|item| info!("Element: {}", item.str_type()));
}