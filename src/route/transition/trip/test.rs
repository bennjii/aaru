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
        LINESTRING (-118.618033 34.166292, -118.623419 34.164641, -118.626895 34.163434)
    };

    let nodes = linestring
        .into_points()
        .into_iter()
        .map(|p| Node::new(p, OsmEntryId::null()))
        .collect::<Vec<_>>();

    let trip = Trip::from(nodes);

    let angle = trip.total_angle();
    assert_relative_eq!(angle, 2.44, max_relative = 0.1);

    let imm_angle = trip.immediate_angle().abs();
    assert_relative_eq!(imm_angle, 0.81, max_relative = 0.1);

    let exp_angle = trip.angular_complexity(SHARED_DISTANCE);
    assert_relative_eq!(exp_angle, 0.95, max_relative = 0.1);
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
    assert_relative_eq!(angle, 195.30, max_relative = 0.1);

    let imm_angle = trip.immediate_angle().abs();
    assert_relative_eq!(imm_angle, 24.41, max_relative = 0.1);

    let exp_angle = trip.angular_complexity(SHARED_DISTANCE);
    assert_relative_eq!(exp_angle, 0.51, max_relative = 0.1);
}

#[test]
fn validate_uturn_expensive() {
    use crate::route::transition::Trip;

    let linestring = wkt! {
        LINESTRING (-118.509833 34.170873, -118.505648 34.170891, -118.51406 34.170908, -118.509849 34.170926, -118.509865 34.172293)
    };

    let nodes = linestring
        .into_points()
        .into_iter()
        .map(|p| Node::new(p, OsmEntryId::null()))
        .collect::<Vec<_>>();

    let trip = Trip::from(nodes);

    let length = trip.length();
    assert_relative_eq!(length, 1698.0, max_relative = 0.1);

    let angle = trip.total_angle();
    assert_relative_eq!(angle, 449.40, max_relative = 0.1);

    let imm_angle = trip.angular_complexity(length);
    assert_relative_eq!(imm_angle, 0.25, max_relative = 0.1);
}
