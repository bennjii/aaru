use routers_network::Entry;
use serde::{Deserialize, Serialize};

use crate::matcher::{Origin, Trip};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "E: Serialize", deserialize = "E: Deserialize<'de>"))]
pub enum Continuation<E>
where
    E: Entry,
{
    /// The trip agrees with the history, and is resumable. The
    /// fresh observations are those beyond the trip which are not
    /// yet matched against the history.
    Resume { trip: Trip<E>, fresh: Vec<Origin> },

    /// The trip contradicts the history (or there was none), and
    /// must be restarted from scratch, the raw event history
    /// is given.
    Restart { fresh: Vec<Origin> },
}

impl<E> Continuation<E>
where
    E: Entry,
{
    /// Reconcile a persisted trip with the committed `history` (chronological, oldest first).
    ///
    /// Overlap is exact [`Origin`] equality — timestamp and position both.
    /// A layer sharing a timestamp with the history but not a position was
    /// solved against data the history contradicts, so it must not resume.
    pub fn reconcile(trip: Option<Trip<E>>, history: &[Origin]) -> Self {
        let Some(mut trip) = trip else {
            return Self::Restart {
                fresh: history.to_vec(),
            };
        };

        let origins = trip.origins();
        let bound = origins.len().min(history.len());
        let overlap = (0..=bound)
            .rev()
            .find(|&k| origins[origins.len() - k..] == history[..k])
            .unwrap_or(0);

        if overlap == 0 {
            return Self::Restart {
                fresh: history.to_vec(),
            };
        }

        trip.tail(overlap);
        Self::Resume {
            trip,
            fresh: history[overlap..].to_vec(),
        }
    }
}
