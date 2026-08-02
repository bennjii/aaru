use alloc::sync::Arc;

use geo::{Destination, Geodesic, Point, Rect};

use crate::{DataPlane, Edge, Node};

pub trait Discovery: DataPlane {
    /// Returns an iterator of edges which fall within the given bounding box.
    fn edges_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = Edge<Node<Self::Entry>>> + Send + 'a>;

    /// Returns an iterator of nodes which fall within the given bounding box.
    fn nodes_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = &'a Node<Self::Entry>> + Send + 'a>;

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
    fn nodes_at_distance<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> Box<dyn Iterator<Item = &'a Node<Self::Entry>> + Send + 'a> {
        let bounds = square_box(point, distance);
        self.nodes_in_box(bounds)
    }

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
    fn edges_at_distance<'a>(
        &'a self,
        point: &Point,
        distance: f64,
    ) -> Box<dyn Iterator<Item = Edge<Node<Self::Entry>>> + Send + 'a> {
        let bounds = square_box(point, distance);
        self.edges_in_box(bounds)
    }

    fn node(&self, id: &Self::Entry) -> Option<&Node<Self::Entry>>;
    fn edge(&self, source: &Self::Entry, target: &Self::Entry) -> Option<Edge<Self::Entry>>;
}

// Forward through `Arc<T>` so shard-managed networks can be held behind
// an `Arc` without losing trait-method access.
impl<T> Discovery for Arc<T>
where
    T: Discovery,
{
    fn edges_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = Edge<Node<Self::Entry>>> + Send + 'a> {
        (**self).edges_in_box(bounds)
    }

    fn nodes_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = &'a Node<Self::Entry>> + Send + 'a> {
        (**self).nodes_in_box(bounds)
    }

    fn node(&self, id: &Self::Entry) -> Option<&Node<Self::Entry>> {
        (**self).node(id)
    }

    fn edge(&self, source: &Self::Entry, target: &Self::Entry) -> Option<Edge<Self::Entry>> {
        (**self).edge(source, target)
    }
}

fn square_box(point: &Point, square_radius: f64) -> Rect<f64> {
    let bottom_right = Geodesic.destination(*point, 135.0, square_radius);
    let top_left = Geodesic.destination(*point, 315.0, square_radius);

    Rect::new(top_left, bottom_right)
}
