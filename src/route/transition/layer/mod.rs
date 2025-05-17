#[doc(hidden)]
pub mod generator;
#[doc(inline)]
pub use generator::*;

use crate::route::transition::candidate::CandidateId;
use geo::Point;

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
