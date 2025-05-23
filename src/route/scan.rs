use geo::{Destination, Geodesic, Haversine, InterpolatableLine, Line, LineLocatePoint, Point};
use rstar::AABB;

use crate::route::Graph;
use crate::route::transition::FatEdge;

use codec::Entry;
use codec::primitive::Node;
#[cfg(feature = "tracing")]
use tracing::Level;

/// Trait containing utility functions to find nodes and edges upon a root structure.
pub trait Scan<Ent>
where
    Ent: Entry,
{
    /// A function which returns an unsorted iterator of [`Node`] references which are within
    /// the provided `distance` of the input [point](Point).
    ///
    /// ### Note
    /// This function implements a square-scan.
    ///
    /// Therefore, it bounds the search to be within a square-radius of the origin. Therefore,
    /// it may not select every node within the supplied distance, or it may select more nodes.
    /// This resolution method is however significantly cheaper than a circular scan, so a wider
    /// or shorter search radius may be required in some use-cases.
    fn nearest_nodes<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = &'a Node<Ent>>
    where
        Ent: 'a;

    /// A function which returns an unsorted iterator of [`FatEdge`] references which are within
    /// the provided `distance` of the input [point](Point).
    ///
    /// ### Note
    /// This function implements a square-scan.
    ///
    /// Therefore, it bounds the search to be within a square-radius of the origin. Therefore,
    /// it may not select every node within the supplied distance, or it may select more nodes.
    /// This resolution method is however significantly cheaper than a circular scan, so a wider
    /// or shorter search radius may be required in some use-cases.
    fn nearest_edges<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = &'a FatEdge<Ent>>
    where
        Ent: 'a;

    /// Searches for, and returns a reference to nearest node from the origin [point](Point).
    /// This node may not exist, and therefore the return type is optional.
    fn nearest_node<'a>(&'a self, point: Point) -> Option<&'a Node<Ent>>
    where
        Ent: 'a;

    /// Returns an iterator over [`Projected`] nodes on each edge within the specified `distance`.
    /// It does so using the [`Scan::nearest_edges`] function.
    ///
    /// ### Note
    /// This is achieved by creating a line from every edge in the iteration, and finding
    /// the closest point upon that line to the source [point](Point).
    /// This is a bounded projection.
    ///
    /// [`Projected`]: https://en.wikipedia.org/wiki/Projection_(linear_algebra)
    fn nearest_projected_nodes<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, &'a FatEdge<Ent>)>
    where
        Ent: 'a;
}

impl<Ent> Scan<Ent> for Graph<Ent>
where
    Ent: Entry,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO, skip(self)))]
    #[inline]
    fn nearest_nodes<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = &'a Node<Ent>>
    where
        Ent: 'a,
    {
        let bottom_right = Geodesic.destination(*point, 135.0, distance);
        let top_left = Geodesic.destination(*point, 315.0, distance);

        let bbox = AABB::from_corners(top_left, bottom_right);
        self.index().locate_in_envelope(&bbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO, skip(self)))]
    #[inline]
    fn nearest_edges<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = &'a FatEdge<Ent>>
    where
        Ent: 'a,
    {
        let bottom_right = Geodesic.destination(*point, 135.0, distance);
        let top_left = Geodesic.destination(*point, 315.0, distance);

        let bbox = AABB::from_corners(top_left, bottom_right);
        self.index_edge().locate_in_envelope(&bbox)
    }

    #[inline]
    fn nearest_node<'a>(&'a self, point: Point) -> Option<&'a Node<Ent>>
    where
        Ent: 'a,
    {
        self.index.nearest_neighbor(&point)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    #[inline]
    fn nearest_projected_nodes<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, &'a FatEdge<Ent>)>
    where
        Ent: 'a,
    {
        // Total overhead of this function is negligible.
        self.nearest_edges(point, distance).filter_map(move |edge| {
            let line = Line::new(edge.source.position, edge.target.position);

            // We locate the point upon the linestring,
            // and then project that fractional (%)
            // upon the linestring to obtain a point
            line.line_locate_point(point)
                .map(|frac| line.point_at_ratio_from_start(&Haversine, frac))
                .map(|point| (point, edge))
        })
    }
}
