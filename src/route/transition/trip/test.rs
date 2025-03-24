use crate::route::transition::Trip;
use crate::{codec::element::variants::common::OsmEntryId, route::transition::trip::Trip};
use approx::assert_relative_eq;

#[test]
fn test_trip() {
    use crate::codec::element::variants::Node;
    use geo::Point;

    let nodes: &[&Node] = &[
        &Node::new(Point::new(0.0, 0.0), OsmEntryId::null()),
        &Node::new(Point::new(0.0, 1.0), OsmEntryId::null()),
        &Node::new(Point::new(1.0, 1.0), OsmEntryId::null()),
        &Node::new(Point::new(1.0, 0.0), OsmEntryId::null()),
    ];

    let trip = Trip::from(nodes);

    let angles = trip.delta_angle();
    assert_relative_eq!(angles[0], 0.0);
    assert_relative_eq!(angles[1], 90.0, max_relative = 1.0);
    assert_relative_eq!(angles[2], 180.0);

    assert_relative_eq!(trip.total_angle(), 180.0);
}
