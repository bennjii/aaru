use crate::codec::element::variants::common::OsmEntryId;
use crate::codec::element::variants::Node;
use crate::route::transition::Trip;
use approx::assert_relative_eq;
use geo::{line_string, wkt};

const SHARED_DISTANCE: f64 = 900.0;

#[test]
fn test_trip() {
    use crate::codec::element::variants::Node;
    use geo::Point;

    let nodes: Vec<Node> = vec![
        Node::new(Point::new(0.0, 0.0), OsmEntryId::null()),
        Node::new(Point::new(0.0, 1.0), OsmEntryId::null()),
        Node::new(Point::new(1.0, 1.0), OsmEntryId::null()),
        Node::new(Point::new(1.0, 0.0), OsmEntryId::null()),
    ];

    let trip = Trip::from(nodes);

    let angles = trip.delta_angle();
    assert_relative_eq!(angles[0], 0.0);
    assert_relative_eq!(angles[1], 90.0, max_relative = 1.0);
    assert_relative_eq!(angles[2], 180.0);

    assert_relative_eq!(trip.total_angle(), 180.0);
}

#[test]
fn validate_segment() {
    use crate::route::transition::Trip;

    let linestring = wkt! {
        LINESTRING (-118.618376 34.166568, -118.624074 34.163894, -118.627379 34.16253, -118.638708 34.158629, -118.642046 34.157824, -118.650757 34.154535, -118.661948 34.150037, -118.664831 34.149782)
    };

    let nodes = linestring
        .into_points()
        .into_iter()
        .map(|p| Node::new(p, OsmEntryId::null()))
        .collect::<Vec<_>>();

    let trip = Trip::from(nodes);

    let angle = trip.total_angle();
    assert_relative_eq!(angle, 42.77, epsilon = 0.1);

    let imm_angle = trip.immediate_angle().abs();
    assert_relative_eq!(imm_angle, 5.35, epsilon = 0.1);

    let exp_angle = trip.angular_complexity(SHARED_DISTANCE);
    assert_relative_eq!(exp_angle, 0.96, epsilon = 0.1);
}

#[test]
fn validate_turning_path() {
    use crate::route::transition::Trip;

    let linestring = wkt! {
        LINESTRING (-118.61829 34.166594, -118.623312 34.164996, -118.62329 34.164073, -118.624127 34.163896, -118.624449 34.163736, -118.625554 34.163461, -118.625929 34.163327, -118.626637 34.162928)
    };

    let nodes = linestring
        .into_points()
        .into_iter()
        .map(|p| Node::new(p, OsmEntryId::null()))
        .collect::<Vec<_>>();

    let trip = Trip::from(nodes);

    let angle = trip.total_angle();
    assert_relative_eq!(angle, 195.30, epsilon = 0.1);

    let imm_angle = trip.immediate_angle().abs();
    assert_relative_eq!(imm_angle, 24.41, epsilon = 0.1);

    let exp_angle = trip.angular_complexity(SHARED_DISTANCE);
    assert_relative_eq!(exp_angle, 0.82, epsilon = 0.1);
}
