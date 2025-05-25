use crate::transition::{Collapse, MatchError};

use codec::Entry;
use geo::LineString;

pub trait Match<E>
where
    E: Entry,
{
    /// TODO: Matches...
    fn map_match(&self, linestring: LineString) -> Result<Collapse<E>, MatchError>;
}
