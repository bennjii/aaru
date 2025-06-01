use std::fmt::Display;
use std::str::FromStr;

use crate::osm::element::{TagString, Tags};
use crate::osm::parsers::Parser;
use crate::osm::primitives::*;

mod subtypes {
    pub const ADVISORY: &'static str = "advisory";
    pub const CONDITIONAL: &'static str = "conditional";
    pub const TYPE: &'static str = "type";
    pub const VARIABLE: &'static str = "variable";

    pub const CONDITION_PATTERN: &'static str = r"\(([^)]+)\)";
}

struct SpeedLimit(Vec<LaneSpeedLimit>);

struct LaneRestriction {
    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:access#Transport_mode_restrictions
    transport_mode: Option<TransportMode>,

    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right
    directionality: Option<Directionality>,
}

impl LaneRestriction {
    fn parse(label: &str) -> LaneRestriction {
        label.split(":").fold(
            LaneRestriction {
                transport_mode: None,
                directionality: None,
            },
            |mut acc, section| LaneRestriction {
                transport_mode: acc.transport_mode.or(TransportMode::from_str(section).ok()),
                directionality: acc
                    .directionality
                    .or(Directionality::from_str(section).ok()),
            },
        )
    }
}

struct SpeedLimitConditions {
    subvariant: LaneRestriction,

    /// The general condition requirements which are separate from
    /// other general conditions. These appear in the post-fix brackets.
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:maxspeed:conditional
    condition: Option<Condition>,
}

/// Represents the speed limit on a singular lane.
///
/// See: https://wiki.openstreetmap.org/wiki/Key:lanes
struct LaneSpeedLimit {
    speed: Option<SpeedValue>,

    restriction: LaneRestriction,
    condition: Option<Condition>,
}

impl LaneSpeedLimit {
    fn parse_tag(tag: &TagString) -> Option<LaneSpeedLimit> {
        let (label, value) = tag.split_once("=")?;

        let restriction = LaneRestriction::parse(label);
        let condition = if label.ends_with(subtypes::CONDITIONAL) {
            let re = regex::Regex::new(subtypes::CONDITION_PATTERN).ok()?;
            let condition_str = re.captures(value)?.get(1)?.as_str();

            Condition::parse(condition_str).ok()
        } else {
            None
        };

        let speed = None; // TODO: !!

        Some(LaneSpeedLimit {
            speed,
            restriction,
            condition,
        })
    }
}

impl Parser<SpeedLimit> for TagString {
    fn parse(tags: Tags) -> Option<SpeedLimit> {
        // Standard structure follows:
        let lanes = tags
            .iter()
            .filter(|(key, _)| key.starts_with(TagString::MAX_SPEED))
            .filter_map(|(_, value)| LaneSpeedLimit::parse_tag(value))
            .collect::<Vec<_>>();

        if lanes.is_empty() {
            return None;
        }

        Some(SpeedLimit(lanes))
    }
}
