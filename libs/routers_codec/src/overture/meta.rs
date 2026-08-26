//! Per-edge metadata for the Overture codec.
//!
//! [`OvertureEdgeMetadata`] implements [`routers_network::Metadata`], the
//! contract generic network consumers evaluate access and attributes
//! through.

use core::num::NonZeroU8;

use routers_network::{Direction, Metadata};
use serde::{Deserialize, Serialize};

use crate::overture::element::Segment;
use crate::overture::parsers::{AccessRestriction, AccessType, RoadClass, SpeedLimit, TravelMode};
use crate::overture::runtime::OvertureTripConfiguration;
use crate::primitive;

/// Routing-relevant attributes of a single segment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OvertureEdgeMetadata {
    /// Always `None`: the Overture schema has no lane-count field.
    pub lane_count: Option<NonZeroU8>,
    pub road_class: Option<RoadClass>,
    /// `subclass: link` — a ramp/slip road.
    pub is_link: bool,
    pub speed_limits: Vec<SpeedLimit>,
    pub access: Vec<AccessRestriction>,
}

impl Metadata for OvertureEdgeMetadata {
    type Raw<'a> = &'a Segment;
    type Runtime = OvertureTripConfiguration;
    type TripContext = primitive::context::TripContext;

    fn pick(raw: Self::Raw<'_>) -> Self {
        Self {
            lane_count: None,
            road_class: raw.road_class,
            is_link: raw.is_link,
            speed_limits: raw.speed_limits.clone(),
            access: raw.access.clone(),
        }
    }

    #[inline]
    fn runtime(ctx: Option<Self::TripContext>) -> Self::Runtime {
        use crate::primitive::transport::TransportMode::*;

        let mut default = OvertureTripConfiguration::default();

        if let Some(ctx) = ctx {
            default.travel_mode = match ctx.transport_mode {
                Car(_) => TravelMode::Car,
                Bus(_) => TravelMode::Bus,
                Truck(_) => TravelMode::Hgv,
                Unspecified => TravelMode::Vehicle,
            };
        }

        default
    }

    #[inline]
    fn accessible(&self, conditions: &Self::Runtime, direction: Direction) -> bool {
        // Accessible by default; inaccessible only if an unconditional
        // `denied` restriction matches both the travel mode and the
        // direction of traversal. Conditional denials (time, purpose,
        // vehicle dimensions, …) hold only under circumstances that cannot
        // be assumed here.
        !self.access.iter().any(|access| {
            !access.conditional
                && access.access_type == AccessType::Denied
                && access.heading.is_none_or(|h| h.direction() == direction)
                && (access.mode.is_empty()
                    || access.mode.iter().any(|m| m.covers(conditions.travel_mode)))
        })
    }
}
