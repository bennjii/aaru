use geo::Point;

use crate::route::transition::candidate::CandidateId;

/// A layer within the transition graph.
///
/// This represents a set of candidate [nodes](#field.nodes),
/// and the [origin](#field.origin) point, from which they originate.
pub struct Layer {
    /// All the candidates detected within the layer, as
    /// positions the [origin](#field.origin) could be matched to.
    pub nodes: Vec<CandidateId>,

    /// The input position within the input to the transition solver.
    ///
    /// This position is consumed by the [`LayerGenerator`](super::LayerGenerator)
    /// to produce candidates for each layer, based on intrinsic location properties.
    pub origin: Point,
}
