//! You may override individual costing strategies
//! in order to apply custom functionality to the
//! transition solver. See the [`Strategy`] trait.
//!
//! ### Using Context
//! Each strategy accepts a context, defined in the
//! generic `Ctx` parameter of the [`Strategy`] trait.
//! Each context is defined statically:
//!
//! - [`TransitionContext`]
//!     Used for the transition costing strategy,
//!     supplies relevant information for the candidates
//!     being routed between, and the optimal trip between them.
//!
//! - [`EmissionContext`]
//!     Used to understand the cost associated with
//!     the selection of a candidate, relative to an optimal
//!     selection on the underlying routing data.
//!
//! There are two static heuristics which must have
//! a strategy defined for them in order to evaluate
//! the costing behind them. A default strategy for each
//! one is defined below.
//!
//! ### Default Strategies:
//! - [`DefaultTransitionCost`]: Transition Cost
//! - [`DefaultEmissionCost`]: Emission Cost
//!
#[doc(hidden)]
pub mod default;
#[doc(hidden)]
pub mod emission;
#[doc(hidden)]
pub mod transition;
#[doc(hidden)]
pub mod util;

#[doc(inline)]
pub use default::*;
#[doc(inline)]
pub use emission::*;
#[doc(inline)]
pub use transition::*;
#[doc(inline)]
pub use util::*;
