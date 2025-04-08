use geo::{line_string, Destination, Geodesic, LineInterpolatePoint, LineLocatePoint, Point};
use petgraph::Direction;
use rstar::AABB;

#[cfg(feature = "tracing")]
use tracing::Level;

use crate::codec::element::variants::Node;
use crate::route::transition::Edge;
use crate::route::Graph;

pub trait Scan {
    /// TODO: Docs
    fn square_scan(&self, point: &Point, distance: f64) -> impl Iterator<Item = &Node>;

    /// TODO: Docs r.e. distinct.
    ///
    /// Finds all **distinct** edges within a square radius of the target position.
    fn nearest_edges(&self, point: &Point, distance: f64) -> impl Iterator<Item = Edge>;

    /// Finds the nearest node to a lat/lng position
    fn nearest_node(&self, point: Point) -> Option<&Node>;

    /// TODO: Docs
    fn nearest_projected_nodes(
        &self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, Edge)>;
}

impl Scan for Graph {
    #[inline]
    fn square_scan(&self, point: &Point, distance: f64) -> impl Iterator<Item = &Node> {
        let bottom_right = Geodesic::destination(*point, 135.0, distance);
        let top_left = Geodesic::destination(*point, 315.0, distance);

        let bbox = AABB::from_corners(top_left, bottom_right);
        self.index().locate_in_envelope(&bbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO, skip(self)))]
    #[inline]
    fn nearest_edges(&self, point: &Point, distance: f64) -> impl Iterator<Item = Edge> {
        // let mut edges_covered = BTreeSet::<EdgeIx>::new();

        self.square_scan(point, distance)
            .flat_map(|node| self.graph.edges_directed(node.id, Direction::Outgoing))
            .map(Edge::from)
        // .filter(move |Edge { id, .. }| !edges_covered.insert(*id))
    }

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
        self.nearest_edges(point, distance)
            .filter_map(|edge| {
                let hashmap = self.hash.read().unwrap();
                let src = hashmap.get(&edge.source)?;
                let trg = hashmap.get(&edge.target)?;

                Some((line_string![src.position.0, trg.position.0], edge))
            })
            .filter_map(move |(linestring, edge)| {
                // We locate the point upon the linestring,
                // and then project that fractional (%)
                // upon the linestring to obtain a point
                linestring
                    .line_locate_point(point)
                    .and_then(|frac| linestring.line_interpolate_point(frac))
                    .map(|point| (point, edge))
            })
    }
}
