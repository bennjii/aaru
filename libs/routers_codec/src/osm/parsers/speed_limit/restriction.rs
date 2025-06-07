use crate::osm::primitives::{Directionality, TransportMode};
use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
pub struct Restriction {
    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:access#Transport_mode_restrictions
    pub(crate) transport_mode: Option<TransportMode>,

    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right
    pub(crate) directionality: Option<Directionality>,
}

impl Restriction {
    pub(crate) fn parse(label: &str) -> Restriction {
        label.split(":").fold(
            Restriction {
                transport_mode: None,
                directionality: None,
            },
            |acc, section| Restriction {
                transport_mode: acc.transport_mode.or(TransportMode::from_str(section).ok()),
                directionality: acc
                    .directionality
                    .or(Directionality::from_str(section).ok()),
            },
        )
    }
}
