use crate::codec::element::variants::Node;
use geo::{Bearing, Distance, Haversine};
use rstar::PointDistance;

/// `Trip`
///
/// Utilities to calculate metadata from trips (Collection of [`Node`]s).
/// Can be created from a slice of nodes.
#[derive(Clone, Copy, Debug)]
pub struct Trip<'a>(&'a [&'a Node]);

impl<'a> From<&'a [&'a Node]> for Trip<'a> {
    fn from(nodes: &'a [&'a Node]) -> Self {
        Trip(nodes)
    }
}

impl Trip<'_> {
    /// Computes the angle between each pair of nodes in the trip.
    /// Allows you to understand the change in heading, aggregatable
    /// using [`Trip::total_angle`] to determine the total variation
    /// exhibited during a trip.
    ///
    /// The returned vector will therefore have a length one less
    /// than the nodes given, and will be empty for a singular node
    /// as there is no delta exhibited.
    ///
    /// This assumes points are given on a great-circle, and uses
    /// Haversine comparisons.
    ///
    /// ### Example
    /// ```rust
    ///  use aaru::codec::element::variants::{Node, OsmEntryId};
    ///  use aaru::route::transition::Trip;
    ///  use geo::Point;
    ///
    ///  // Create some nodes
    ///  let nodes: &[&Node] = &[
    ///     &Node::new(Point::new(0.0, 0.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(0.0, 1.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(1.0, 1.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(1.0, 0.0), OsmEntryId::null()),
    ///  ];
    ///
    ///  // Form a trip from these nodes
    ///  let trip = Trip::from(nodes);
    ///
    ///  // Calculate the delta angle exhibited
    ///  println!("{:?}", trip.delta_angle());
    ///  // # [0, 90, 180]
    /// ```
    pub fn delta_angle(&self) -> Vec<f64> {
        self.0
            .windows(2)
            .map(|entries| {
                if let [a, b] = entries {
                    // Returns the bearing relative to due-north
                    Haversine::bearing(a.position, b.position)
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>()
    }

    /// Computes the sum of angle differences within a trip.
    /// Useful as a quantifiable heuristic to determine how "non-direct" a trip is.
    ///
    /// ### Example
    /// ```rust
    ///  use aaru::codec::element::variants::{Node, OsmEntryId};
    ///  use aaru::route::transition::Trip;
    ///  use geo::Point;
    ///
    ///  // Create some nodes
    ///  let nodes: &[&Node] = &[
    ///     &Node::new(Point::new(0.0, 0.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(0.0, 1.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(1.0, 1.0), OsmEntryId::null()),
    ///     &Node::new(Point::new(1.0, 0.0), OsmEntryId::null()),
    ///  ];
    ///
    ///  // Form a trip from these nodes
    ///  let trip = Trip::from(nodes);
    ///
    ///  // Calculate the total angle exhibited
    ///  println!("{}", trip.total_angle());
    ///  // # 180
    /// ```
    pub fn total_angle(&self) -> f64 {
        self.delta_angle()
            .windows(2)
            .map(|bearings| {
                if let [prev, curr] = bearings {
                    let mut turn_angle = (curr - prev).abs();

                    // Normalize to [-180, 180] degrees
                    if turn_angle > 180.0 {
                        turn_angle -= 360.0;
                    } else if turn_angle < -180.0 {
                        turn_angle += 360.0;
                    }

                    turn_angle.abs()
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// Calculates the "immediate" (or average) angle within a trip.
    /// Used to understand the "average" angular movement per move.
    ///
    /// It is important to understand this is intrinsically weighted
    /// by the number of nodes, such that denser areas will reduce
    /// this weighting and vice versa.
    pub fn immediate_angle(&self) -> f64 {
        self.total_angle() / (self.0.len() as f64)
    }

    /// Returns the length of the trip in meters, calculated
    /// by the distance between each entry in the trip.
    pub fn length(&self) -> f64 {
        self.0.windows(2).fold(0.0, |length, node| {
            if let [a, b] = node {
                return length + Haversine::distance(a.position, b.position);
            }

            length
        })
    }
}
