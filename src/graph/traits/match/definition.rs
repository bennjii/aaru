use crate::transition::{Collapse, MatchError, PredicateCache};

use codec::Entry;
use geo::LineString;
use std::sync::{Arc, Mutex};

pub trait Match<E>
where
    E: Entry,
{
    /// TODO: Matches...
    fn map_match(
        &self,
        linestring: LineString,
        // TODO: Make this an associated member element (Mutex in args is off-putting!)
        cache: Arc<Mutex<PredicateCache<E>>>,
    ) -> Result<Collapse<E>, MatchError>;
}
