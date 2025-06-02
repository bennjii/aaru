use std::ops::Deref;
use std::str::FromStr;

use crate::osm::element::{TagString, Tags};
use crate::osm::parsers::Parser;
use crate::osm::primitives::speed::Speed;
use crate::osm::primitives::*;

mod subtypes {
    pub const LANES: &str = "lanes";

    pub const CONDITION_PATTERN: &str = r"\(([^)]+)\)";
    pub const VALUE_PATTERN: &str = r"^\s*(\d+)(?:\s*([^\s(]+))?";
}

#[derive(Clone, Copy, Debug)]
pub struct Restriction {
    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:access#Transport_mode_restrictions
    transport_mode: Option<TransportMode>,

    /// ...
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Forward_%26_backward,_left_%26_right
    directionality: Option<Directionality>,
}

impl Restriction {
    fn parse(label: &str) -> Restriction {
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

/// Defines a speed limit which may contain a conditional element.
/// For example:
///  - `maxspeed=50 @ (Mo-Fr 06:00-22:00)`
///  - `maxspeed:lanes=100 @ (22:00-06:00)|60`
///
/// This represents the individual `number <unit> @ (...)` value.
/// This may be spread across a lane representation in the SpeedLimit
/// structure.
#[derive(Clone, Debug)]
pub struct PossiblyConditionalSpeedLimit {
    /// Represents the speed limit on a singular lane.
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:lanes
    pub speed: SpeedValue,

    /// The general condition requirements which are separate from
    /// other general conditions. These appear in the post-fix brackets.
    ///
    /// See: https://wiki.openstreetmap.org/wiki/Key:maxspeed:conditional
    pub condition: Option<Condition>,
}

impl PossiblyConditionalSpeedLimit {
    /// Parses the condition on a speed limit value.
    ///
    /// It extracts the condition value and passes it to the `Condition`
    /// structure, attempting to parse the content. None is returned if
    /// this operation fails, or no condition exists on the input.
    fn parse_condition(value: &str) -> Option<Condition> {
        let re = regex::Regex::new(subtypes::CONDITION_PATTERN).ok()?;
        let condition_str = re.captures(value)?.get(1)?.as_str();

        Condition::parse(condition_str).ok()
    }

    /// Parses the speed value (unit and number) on a speed limit value.
    ///
    /// It extracts the speed limit and passes it to the `Speed` structure,
    /// attempting to parse the content inside. None is returned if this
    /// operation fails.
    fn parse_speed(value: &str) -> Option<SpeedValue> {
        let re = regex::Regex::new(subtypes::VALUE_PATTERN).ok()?;
        let captures = re.captures(value)?;

        let (value, unit) = (
            captures
                .get(1)
                .map(|v| v.as_str().to_lowercase())
                .unwrap_or_default(),
            captures
                .get(2)
                .map(|v| v.as_str().to_lowercase())
                .unwrap_or_default(),
        );

        SpeedValue::parse(value, unit)
    }

    /// Parses the speed limit from an individual tag value,
    /// like `50 @ (Mo-Fr 06:00-22:00)`.
    ///
    /// If broken down into individual lanes, this function
    /// can be applied to each individual lane's value.
    ///
    /// For example, this function does not accept `50 @ (...)|50 @ (...)`.
    /// Instead, pass each entry independently.
    pub fn parse(value: &str) -> Option<Self> {
        // Ignore values such as the middle lane in `20||20`,
        // where the lane does not contain an entry.
        if value.is_empty() {
            return None;
        }

        let condition = Self::parse_condition(value);
        let speed = Self::parse_speed(value)?;

        Some(PossiblyConditionalSpeedLimit { condition, speed })
    }
}

#[derive(Clone, Debug)]
pub struct PerLaneSpeedLimit(Vec<Option<PossiblyConditionalSpeedLimit>>);

impl Deref for PerLaneSpeedLimit {
    type Target = Vec<Option<PossiblyConditionalSpeedLimit>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PerLaneSpeedLimit {
    pub fn in_kmh(&self) -> Vec<Option<Speed>> {
        self.iter()
            .map(|lane| lane.as_ref().and_then(|lan| lan.speed.in_kmh()))
            .collect::<Vec<_>>()
    }
}

#[derive(Clone, Debug)]
pub enum SpeedLimitVariant {
    /// Applies to every lane within the way, and is
    /// therefore non-dependent on lanes.
    Blanket(PossiblyConditionalSpeedLimit),

    /// Contains an array with size `N` where `N` is the
    /// number of lanes. Each `None` value in the
    /// vector is for lanes in which the limit is not
    /// specified.
    PerLane(PerLaneSpeedLimit),
}

#[derive(Clone, Debug)]
pub struct SpeedLimit {
    pub restriction: Restriction,
    pub limit: SpeedLimitVariant,
}

impl SpeedLimit {
    fn parse_tag(label: &TagString, value: &TagString) -> Option<Self> {
        let restriction = Restriction::parse(label);

        // Is lane-based (i.e. maxspeed:lanes=50|20|10)
        // Cannot perform an `ends_with(..)` since the
        // "conditional" flag is the subsuming suffix.
        // I.e. maxspeed:lanes:conditional=20 @ ()|10 @ (Mo-Fr 10:00-12:00)
        let limit = if label.contains(subtypes::LANES) {
            let per_lane_limit = value
                .split_terminator("|")
                .map(PossiblyConditionalSpeedLimit::parse)
                .collect::<Vec<_>>();

            SpeedLimitVariant::PerLane(PerLaneSpeedLimit(per_lane_limit))
        } else {
            let as_str = value.as_str();
            let speed_limit = PossiblyConditionalSpeedLimit::parse(as_str)?;
            SpeedLimitVariant::Blanket(speed_limit)
        };

        Some(Self { limit, restriction })
    }
}

struct SpeedLimitCollection(Vec<SpeedLimit>);

impl Deref for SpeedLimitCollection {
    type Target = Vec<SpeedLimit>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Parser for SpeedLimitCollection {
    fn parse(tags: Tags) -> Option<Self> {
        // Standard structure follows:
        let known_limits = tags
            .iter()
            .filter(|(key, _)| key.starts_with(TagString::MAX_SPEED))
            .filter_map(|(l, v)| SpeedLimit::parse_tag(l, v))
            .collect::<Vec<_>>();

        if known_limits.is_empty() {
            return None;
        }

        Some(Self(known_limits))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::osm::Parser;
    use crate::osm::element::{TagString, Tags};
    use crate::osm::primitives::condition::{ConditionType, TimeDateCondition};
    use crate::osm::primitives::opening_hours::{Time, TimeRange, Weekday, WeekdayRange};
    use crate::osm::speed_limit::SpeedLimitVariant::{Blanket, PerLane};
    use crate::osm::speed_limit::{SpeedLimit, SpeedLimitCollection};

    #[cfg(test)]
    fn parse_singular(key: &str, value: &str) -> SpeedLimit {
        let mut tags = HashMap::new();
        tags.insert(TagString::from(key), TagString::from(value));

        let as_tags = Tags::new(tags);
        let limit = SpeedLimitCollection::parse(as_tags);
        assert!(limit.is_some(), "must parse successfully");

        let limits = limit.unwrap().0;
        assert_eq!(1, limits.len(), "must contain a singular limit");

        limits[0].clone()
    }

    #[test]
    fn test_parsing_speed_limit() {
        let parsed_limit = parse_singular("maxspeed", "50");

        assert!(
            parsed_limit.restriction.transport_mode.is_none(),
            "must not specify a transport mode"
        );
        assert!(
            parsed_limit.restriction.directionality.is_none(),
            "must not specify a directionality"
        );

        matches!(
            parsed_limit.limit,
            Blanket(limit) if limit.speed.in_kmh().is_some_and(|limit| limit == 50)
        );
    }

    #[test]
    fn test_parsing_conditional() {
        let parsed_limit = parse_singular("maxspeed:conditional", "130 @ (19:00-06:00)");

        assert!(
            parsed_limit.restriction.transport_mode.is_none(),
            "must not specify a transport mode"
        );
        assert!(
            parsed_limit.restriction.directionality.is_none(),
            "must not specify a directionality"
        );

        assert!(
            matches!(parsed_limit.limit, Blanket(..)),
            "must be a blanket assignment"
        );
        assert!(matches!(parsed_limit.limit, Blanket(ref limit) if {
            limit.speed
                .in_kmh()
                .is_some_and(|limit| limit == 130)
        }));
        assert!(matches!(parsed_limit.limit, Blanket(ref limit) if {
            let condition_type =  limit.condition.clone().unwrap().condition_type;
            matches!(condition_type, ConditionType::TimeDate(TimeDateCondition {
                opening_hours, ..
            }) if {
                // Overnight ruling: 19:00-06:00
                opening_hours.rules[0].times[0] == TimeRange {
                    start: Time {
                        hour: 19,
                        minute: 0,
                    },
                    end: Time {
                        hour: 6,
                        minute: 0,
                    }
                }
            })
        }));
    }

    #[test]
    fn test_parsing_lanes() {
        let parsed_limit = parse_singular("maxspeed:lanes", "100|80|80|80|80|80");

        assert!(
            parsed_limit.restriction.transport_mode.is_none(),
            "must not specify a transport mode"
        );
        assert!(
            parsed_limit.restriction.directionality.is_none(),
            "must not specify a directionality"
        );

        matches!(parsed_limit.limit, PerLane(..));

        if let PerLane(lanes) = parsed_limit.limit {
            let speeds = lanes.in_kmh();

            assert_eq!(speeds.len(), 6, "must contain exactly 6 lanes");
            assert_eq!(
                speeds.as_slice(),
                &[Some(100), Some(80), Some(80), Some(80), Some(80), Some(80)],
                "must parse limits correctly"
            );
        }
    }

    #[test]
    fn test_parsing_lanes_with_missing() {
        let parsed_limit = parse_singular("maxspeed:lanes", "|50");

        matches!(parsed_limit.limit, PerLane(..));

        if let PerLane(lanes) = parsed_limit.limit {
            let speeds = lanes.in_kmh();

            assert_eq!(speeds.len(), 2, "must contain exactly 2 lanes");
            assert_eq!(
                speeds.as_slice(),
                &[None, Some(50)],
                "must parse limits correctly"
            );
        }
    }

    #[test]
    fn test_parsing_lanes_mph() {
        let parsed_limit =
            parse_singular("maxspeed:lanes:conditional", "65 mph|65 mph|65 mph|25 mph");

        matches!(parsed_limit.limit, PerLane(..));

        if let PerLane(lanes) = parsed_limit.limit {
            let speeds = lanes.in_kmh();

            assert_eq!(speeds.len(), 4, "must contain exactly 4 lanes");
            assert_eq!(
                speeds.as_slice(),
                &[Some(104), Some(104), Some(104), Some(40)],
                "must parse limits correctly"
            );
        }
    }

    #[test]
    fn test_parsing_lanes_conditional() {
        let parsed_limit = parse_singular(
            "maxspeed:lanes:conditional",
            "100 @ (22:00-06:00)|40 @ (Mo-Fr 07:00-9:00,16:00-20:00)|60",
        );

        matches!(parsed_limit.limit, PerLane(..));

        if let PerLane(lanes) = parsed_limit.limit {
            let speeds = lanes.in_kmh();

            assert_eq!(speeds.len(), 3, "must contain exactly 3 lanes");
            assert_eq!(
                speeds.as_slice(),
                &[Some(100), Some(40), Some(60)],
                "must parse limits correctly"
            );

            // Expecting 22:00-06:00 condition
            let lane_one = lanes[0].as_ref().unwrap();
            let lane_one_condition = lane_one.condition.as_ref();

            assert!(lane_one_condition.is_some(), "must specify a condition");
            assert!(matches!(
                lane_one_condition.unwrap().clone().condition_type,
                ConditionType::TimeDate(date) if {
                    date.opening_hours.rules[0].times[0] == TimeRange {
                        start: Time { hour: 22, minute: 0 },
                        end: Time { hour: 6, minute: 0}
                    }
                }
            ));

            // Expecting Mo-Fr 07:00-9:00,16:00-20:00 condition
            let lane_two = lanes[1].as_ref().unwrap();
            let lane_two_condition = lane_two.condition.as_ref();

            assert!(lane_two_condition.is_some(), "must specify a condition");
            assert!(matches!(
                lane_two_condition.unwrap().clone().condition_type,
                ConditionType::TimeDate(date) if {
                    let rule =  date.opening_hours.rules[0].clone();

                    rule.times == vec![
                        TimeRange {
                            start: Time { hour: 7, minute: 0 },
                            end: Time { hour: 9, minute: 0 },
                        },
                        TimeRange {
                            start: Time { hour: 16, minute: 0},
                            end: Time { hour: 20, minute: 0},
                        }
                    ] && rule.weekdays == Some(WeekdayRange::Range(Weekday::Monday, Weekday::Friday))
                }
            ));

            // Expecting no condition
            let lane_three = lanes[2].as_ref().unwrap();
            let lane_three_condition = lane_three.condition.as_ref();

            assert!(
                lane_three_condition.is_none(),
                "must not specify a condition"
            );
        }
    }
}
