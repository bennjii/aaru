use crate::transition::candidate::*;
use codec::Entry;
use std::ops::Deref;

use geo::Point;

/// A route representing the parsed output from a function
/// passed through the transition graph.
pub struct RoutedPath<E, Meta>
where
    E: Entry,
{
    /// The exactly-routed elements.
    ///
    /// For a map-match request, these are the values which line up with the inputs
    /// for a one-to-one match. I.e. there is a discretized point for every input point.
    pub discretized: Path<E, Meta>,

    /// The interpolated elements.
    ///
    /// These points are the full interpreted trip, consisting of every turn and roadway
    /// the algorithm has assumed as a part of the path taken. This is useful for visualising
    /// a trip by "recovering" lost information, or understanding subtle details such as
    /// when the route left or joined a highway.
    pub interpolated: Path<E, Meta>,
}

impl<E, Meta> RoutedPath<E, Meta>
where
    E: Entry,
{
    pub fn new(_collapsed_path: CollapsedPath<E>) -> Self {
        todo!();
    }
}

/// A representation of a path taken.
/// Consists of an array of [PathElement]s, containing relevant information for positioning.
pub struct Path<E, Meta>
where
    E: Entry,
{
    /// The elements which construct the path.
    elements: Vec<PathElement<E, Meta>>,
}

impl<E, Meta> Deref for Path<E, Meta>
where
    E: Entry,
{
    type Target = Vec<PathElement<E, Meta>>;

    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

/// An element within a path, consisting of the [Point] the
/// element represents within the path, as well as metadata (Meta)
/// for the path element, and the edge within the source network at
/// which the element exists.
pub struct PathElement<E, Meta>
where
    E: Entry,
{
    pub point: Point,
    pub edge: Edge<E>,

    metadata: Meta,
}
