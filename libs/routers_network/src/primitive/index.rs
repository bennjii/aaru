//! Packed spatial indexing shared by the network implementations.
//!
//! Wraps a [`geo_index`] Hilbert-packed R-tree (a flat `Vec<u8>` of
//! bounding boxes + a flat index array — one allocation, no per-node
//! pointers) together with the payload row table that search results
//! index into.
//!
//! The tree itself stores **only** envelopes: every query returns
//! insertion-order row indices (via `u32`), so [`RowIndex`] pairs the
//! tree with the `rows` it was built from and resolves them back to
//! typed payload references.

use geo::{Point, Rect};
use geo_index::rtree::sort::HilbertSort;
use geo_index::rtree::{RTree, RTreeBuilder, RTreeIndex};

/// A Hilbert-packed R-tree paired with the payload rows it indexes.
///
/// [`RowIndex::build`] takes ownership of the payload rows and derives
/// their envelopes, so tree row `i` always corresponds to `rows[i]`.
#[derive(Debug)]
pub struct RowIndex<R> {
    tree: RTree<f64>,
    rows: Vec<R>,
}

impl<R> Default for RowIndex<R> {
    fn default() -> Self {
        Self {
            tree: RTreeBuilder::new(0).finish::<HilbertSort>(),
            rows: Vec::new(),
        }
    }
}

impl<R> RowIndex<R> {
    /// Build an index over `rows`, deriving `(min, max)` corner envelopes
    /// per row using `envelope`.
    ///
    /// Build cost is `O(n log n)` (Hilbert sort); the resulting tree is
    /// static — rebuild it when the payload set changes.
    pub fn build(mut rows: Vec<R>, envelope: impl Fn(&R) -> (Point, Point)) -> Self {
        // Filtered `collect` leaves growth slack in the vec (capacity can
        // be up to 2× len); give the heap back.
        rows.shrink_to_fit();

        let mut builder = RTreeBuilder::new(rows.len() as u32);
        for row in &rows {
            let (min, max) = envelope(row);
            builder.add(min.x(), min.y(), max.x(), max.y());
        }

        Self {
            tree: builder.finish::<HilbertSort>(),
            rows,
        }
    }

    /// Number of indexed rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// `true` if no rows are indexed.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// All rows whose envelope intersects `bounds`, in no defined order.
    ///
    /// Materialises a `Vec<u32>` of matching rows internally (one small
    /// allocation per call).
    pub fn search<'a>(&'a self, bounds: Rect<f64>) -> impl Iterator<Item = &'a R> + 'a {
        let (min, max) = (bounds.min(), bounds.max());
        self.tree
            .search(min.x, min.y, max.x, max.y)
            .into_iter()
            .filter_map(|idx| self.rows.get(idx as usize))
    }

    /// The single nearest row to `point` under planar (Euclidean)
    /// distance over coordinate degrees.
    pub fn nearest(&self, point: &Point) -> Option<&R> {
        self.tree
            .neighbors_coord(&geo::Coord::from(point.x_y()), Some(1), None)
            .first()
            .and_then(|&idx| self.rows.get(idx as usize))
    }
}

/// Envelope `(min, max)` corners spanning two points (e.g. an edge's
/// endpoints), order-independent.
#[inline]
pub fn envelope_of(a: Point, b: Point) -> (Point, Point) {
    (
        Point::new(a.x().min(b.x()), a.y().min(b.y())),
        Point::new(a.x().max(b.x()), a.y().max(b.y())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random coordinate set.
    fn sample_points(n: usize) -> Vec<Point> {
        let mut pts = Vec::with_capacity(n);
        let mut x: u64 = 0x9E3779B97F4A7C15;
        for _ in 0..n {
            // xorshift64*
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            let lon =
                -180.0 + ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) % 360_000) as f64 / 1000.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            let lat =
                -80.0 + ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) % 160_000) as f64 / 1000.0;
            pts.push(Point::new(lon, lat));
        }
        pts
    }

    /// `search` must return exactly the rows the naive envelope-intersect
    /// scan returns — no misses, no false positives.
    #[test]
    fn search_matches_brute_force() {
        let pts = sample_points(2_000);
        let index = RowIndex::build(pts.clone(), |p| (*p, *p));

        for (cx, cy, d) in [
            (0.0, 0.0, 40.0),
            (10.0, -5.0, 12.5),
            (100.0, 30.0, 1.0),
            (-170.0, -70.0, 55.0),
        ] {
            let bounds = Rect::new(Point::new(cx - d, cy - d), Point::new(cx + d, cy + d));

            let mut expected: Vec<Point> = pts
                .iter()
                .copied()
                .filter(|p| {
                    p.x() >= bounds.min().x
                        && p.x() <= bounds.max().x
                        && p.y() >= bounds.min().y
                        && p.y() <= bounds.max().y
                })
                .collect();
            expected.sort_by(|a, b| a.x_y().0.total_cmp(&b.x_y().0));

            let mut got: Vec<Point> = index.search(bounds).copied().collect();
            got.sort_by(|a, b| a.x_y().0.total_cmp(&b.x_y().0));

            assert_eq!(expected, got, "box cx={cx} cy={cy} d={d}");
        }
    }

    /// `nearest` must return the brute-force nearest row.
    #[test]
    fn nearest_matches_brute_force() {
        let pts = sample_points(1_000);
        let index = RowIndex::build(pts.clone(), |p| (*p, *p));

        for q in sample_points(50) {
            let expected = pts
                .iter()
                .min_by(|a, b| {
                    let d2 = |p: &Point| (p.x() - q.x()).powi(2) + (p.y() - q.y()).powi(2);
                    d2(a).total_cmp(&d2(b))
                })
                .copied();
            let got = index.nearest(&q).copied();

            // Exact ties are broken arbitrarily by both implementations,
            // so compare by distance rather than identity.
            let d2 = |p: &Point| (p.x() - q.x()).powi(2) + (p.y() - q.y()).powi(2);
            assert_eq!(expected.map(|p| d2(&p)), got.map(|p| d2(&p)), "query {q:?}");
        }
    }

    /// Empty indices must never panic and must yield nothing.
    #[test]
    fn empty_index() {
        use alloc::vec;
        let index: RowIndex<Point> = RowIndex::default();
        assert!(index.is_empty());
        assert_eq!(
            index
                .search(Rect::new(Point::new(-1.0, -1.0), Point::new(1.0, 1.0)))
                .count(),
            0
        );
        assert!(index.nearest(&Point::new(0.0, 0.0)).is_none());

        let single = RowIndex::build(vec![Point::new(1.0, 1.0)], |p| (*p, *p));
        assert_eq!(
            single.nearest(&Point::new(9.0, 9.0)),
            Some(&Point::new(1.0, 1.0))
        );
    }
}
