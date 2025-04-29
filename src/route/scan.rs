use geo::{
    Destination, Distance, Geodesic, Haversine, InterpolatableLine, Line, LineLocatePoint, Point,
};
use itertools::Itertools;
use rstar::AABB;

use crate::codec::element::variants::Node;
use crate::route::transition::{Edge, FatEdge};
use crate::route::Graph;
#[cfg(feature = "tracing")]
use tracing::Level;

pub trait Scan {
    /// TODO: Docs
    fn nearest_nodes(&self, point: &Point, distance: f64) -> impl Iterator<Item = &Node>;

    /// TODO: Docs r.e. distinct.
    /// Finds all edges within a set square radius
    fn nearest_edges(&self, point: &Point, distance: f64) -> impl Iterator<Item = &FatEdge>;

    /// Finds the nearest node to a lat/lng position
    fn nearest_node(&self, point: Point) -> Option<&Node>;

    /// TODO: Docs
    fn nearest_projected_nodes(
        &self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, Edge)>;

    fn nearest_projected_nodes_sorted(
        &self,
        point: Point,
        search_distance: f64,
        filter_distance: f64,
    ) -> impl Iterator<Item = (Point, Edge, f64)>;

    /// Finds all **distinct** edges within a square radius of the target position.
    fn edge_distinct_nearest_projected_nodes_sorted(
        &self,
        point: Point,
        search_distance: f64,
        filter_distance: f64,
    ) -> impl Iterator<Item = (Point, Edge, f64)>;
}

impl Scan for Graph {
    #[inline]
    fn nearest_nodes(&self, point: &Point, distance: f64) -> impl Iterator<Item = &Node> {
        let bottom_right = Geodesic.destination(*point, 135.0, distance);
        let top_left = Geodesic.destination(*point, 315.0, distance);

        let bbox = AABB::from_corners(top_left, bottom_right);
        self.index().locate_in_envelope(&bbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO, skip(self)))]
    #[inline]
    fn nearest_edges(&self, point: &Point, distance: f64) -> impl Iterator<Item = &FatEdge> {
        let bottom_right = Geodesic.destination(*point, 135.0, distance);
        let top_left = Geodesic.destination(*point, 315.0, distance);

        let bbox = AABB::from_corners(top_left, bottom_right);
        self.index_edge().locate_in_envelope(&bbox)
    }

    #[inline]
    fn nearest_node(&self, point: Point) -> Option<&Node> {
        self.index.nearest_neighbor(&point)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    #[inline]
    fn nearest_projected_nodes(
        &self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, Edge)> {
        // Total overhead of this function is negligible.
        self.nearest_edges(point, distance).filter_map(move |edge| {
            let line = Line::new(edge.source.position, edge.target.position);

            // We locate the point upon the linestring,
            // and then project that fractional (%)
            // upon the linestring to obtain a point
            line.line_locate_point(point)
                .map(|frac| line.point_at_ratio_from_start(&Haversine, frac))
                .map(|point| (point, edge.thin()))
        })
    }

    #[inline]
    fn nearest_projected_nodes_sorted(
        &self,
        source: Point,
        search_distance: f64,
        filter_distance: f64,
    ) -> impl Iterator<Item = (Point, Edge, f64)> {
        self.nearest_projected_nodes(&source, search_distance)
            .filter_map(move |(point, edge)| {
                let distance = Haversine.distance(point, source);

                if distance < filter_distance {
                    Some((point, edge, distance))
                } else {
                    None
                }
            })
            .sorted_by(|(_, _, a), (_, _, b)| a.total_cmp(b))
    }

    #[inline]
    fn edge_distinct_nearest_projected_nodes_sorted(
        &self,
        point: Point,
        search_distance: f64,
        filter_distance: f64,
    ) -> impl Iterator<Item = (Point, Edge, f64)> {
        // let mut edges_covered = BTreeSet::<DirectionAwareEdgeId>::new();

        // Removes all candidates from the **sorted** projected nodes which lie on the same WayID,
        // such that we only keep the closest node for every way, and considering direction a
        // WayID as a composite of the underlying map ID and the direction of the points within
        // the way.
        self.nearest_projected_nodes_sorted(point, search_distance, filter_distance)
        // TODO: Revisit - Has problems creating *correct* routes.
        // .filter(move |(_, Edge { id, .. })| edges_covered.insert(*id))
    }
}
