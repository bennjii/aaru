use alloc::borrow::Cow;

use crate::candidate::*;
use crate::primitives::{Reachable, ResolutionMethod};
use geo::{LineString, Point};
use routers_network::Entry;
use routers_network::Network;

/// A solved map-match: the chosen candidate per input point, plus the routed
/// path between them.
pub struct CollapsedPath<'a, E>
where
    E: Entry,
{
    /// Total cost of the chosen route — a confidence indicator, not a distance.
    pub cost: u32,

    /// The chosen candidate per layer, in order. Resolve to [`Candidate`]s with
    /// [`matched`](Self::matched).
    pub route: Vec<CandidateRef>,

    /// One [`Reachable`] per hop, each holding the routed path between consecutive
    /// chosen candidates. Render it with [`interpolated`](Self::interpolated).
    pub interpolated: Vec<Reachable<E>>,

    /// The candidate store resolving the [`CandidateRef`]s in [`route`](Self::route).
    pub candidates: Cow<'a, CandidateStore<E>>,
}

impl<E> CollapsedPath<'_, E>
where
    E: Entry,
{
    /// Detach from the borrowed trip by cloning the candidate store (a no-op
    /// when it is already owned).
    pub fn into_owned<'b>(self) -> CollapsedPath<'b, E>
    where
        E: 'b,
    {
        CollapsedPath {
            cost: self.cost,
            route: self.route,
            interpolated: self.interpolated,
            candidates: Cow::Owned(self.candidates.into_owned()),
        }
    }

    /// The chosen [`Candidate`] for each matched input point.
    pub fn matched(&self) -> Vec<Candidate<E>> {
        self.route
            .iter()
            .filter_map(|r| self.candidates.candidate(r))
            .collect::<Vec<_>>()
    }

    /// The matched candidate positions as a [`LineString`] (one point per input).
    pub fn collapsed(&self) -> LineString {
        self.matched()
            .iter()
            .map(|candidate| candidate.position)
            .collect::<LineString>()
    }

    /// The road geometry driven across hop `hop` (between matched layers
    /// `hop` and `hop + 1`), exclusive of both endpoints' matched positions:
    /// the current edge's exit node, any routed intermediate nodes, and the
    /// next edge's entry node, with the shared seam nodes deduplicated.
    ///
    /// Empty for a same-edge hop — travel never leaves the edge, so the
    /// endpoints alone describe it — and for a `hop` out of range.
    pub fn hop_geometry(&self, hop: usize, map: &impl Network<Entry = E>) -> Vec<Point> {
        let Some(reachable) = self.interpolated.get(hop) else {
            return Vec::new();
        };
        if !matches!(reachable.resolution_method, ResolutionMethod::Standard) {
            return Vec::new();
        }

        let exit = self
            .candidates
            .candidate(&reachable.source)
            .map(|candidate| candidate.edge.target);
        let entry = self
            .candidates
            .candidate(&reachable.target)
            .map(|candidate| candidate.edge.source);

        // Consecutive bridge edges share their endpoints with each other and
        // with the exit/entry nodes, so the seams dedup away.
        let mut points: Vec<Point> = exit
            .into_iter()
            .chain(reachable.path_nodes())
            .chain(entry)
            .filter_map(|node| map.point(&node))
            .collect();
        points.dedup();
        points
    }

    /// The full driven path as a [`LineString`] — the matched positions with
    /// each hop's [`geometry`](Self::hop_geometry) filled in, showing the
    /// turns taken.
    pub fn interpolated(&self, map: &impl Network<Entry = E>) -> LineString {
        let matched = self.matched();

        let mut points = Vec::new();
        if let Some(first) = matched.first() {
            points.push(first.position);
        }
        for (index, target) in matched.iter().enumerate().skip(1) {
            points.extend(self.hop_geometry(index - 1, map));
            points.push(target.position);
        }

        points.into_iter().collect::<LineString>()
    }
}
