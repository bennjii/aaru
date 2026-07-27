use chrono::{DateTime, Utc};
use geo::Point;
use routers_network::{Entry, Metadata};
use routers_shard::{Geohash, GeohashStrategy, ShardingStrategy};
use routers_transition::candidate::RoutedPath;
use routers_transition::matcher::{Continuation, Trip};
use serde::{Deserialize, Serialize};

use buffa::Message;
use schema::proto::routers::realtime::v1 as proto;

use crate::bus::{Wire, postcard_wire};
use crate::store::Storable;

/// Declare a compact wire identifier. The upstream string ids are stepped
/// down to these at the ingest boundary (see the replay binary), so the
/// strings never cross the wire again.
macro_rules! wire_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub u32);

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

wire_id! {
    /// Identifies one vehicle across its events, history, and matches.
    VehicleId
}
wire_id! {
    /// Identifies the trip an event was observed under.
    TripId
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "E: Serialize", deserialize = "E: Deserialize<'de>"))]
pub struct MatchContext<E: Entry> {
    pub continuation: Continuation<E>,
    pub vehicle_id: VehicleId,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchResult<E: Entry, M: Metadata> {
    pub path: RoutedPath<E, M>,
    pub vehicle_id: VehicleId,
    pub trip: Trip<E>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub trip_id: TripId,
    pub vehicle_id: VehicleId,

    /// When the observation was made. Serialized as microseconds since the
    /// Unix epoch on the wire.
    #[serde(with = "chrono::serde::ts_microseconds")]
    pub timestamp: DateTime<Utc>,

    pub point: Point,
}

// The match control plane is Rust-internal: postcard on the wire.
postcard_wire!(MatchContext<E: Entry>);
postcard_wire!(MatchResult<E: Entry, M: Metadata>);

/// The ingest surface crosses the bus as protobuf
/// (`routers.realtime.v1.Payload`), so producers in any language can
/// publish raw events against the schema.
impl Wire for Payload {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(proto::Payload::from(self).encode_to_vec())
    }

    fn decode(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self::from(proto::Payload::decode_from_slice(bytes)?))
    }
}

impl From<&Payload> for proto::Payload {
    fn from(payload: &Payload) -> Self {
        proto::Payload {
            trip_id: payload.trip_id.0,
            vehicle_id: payload.vehicle_id.0,
            timestamp: buffa::MessageField::some(buffa_types::google::protobuf::Timestamp::from_unix(
                payload.timestamp.timestamp(),
                payload.timestamp.timestamp_subsec_nanos() as i32,
            )),
            point: buffa::MessageField::some(schema::proto::routers::model::v1::Coordinate {
                longitude: payload.point.x(),
                latitude: payload.point.y(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl From<proto::Payload> for Payload {
    fn from(payload: proto::Payload) -> Self {
        let point = payload.point.into_option().unwrap_or_default();
        let timestamp = payload.timestamp.into_option().unwrap_or_default();

        Payload {
            trip_id: TripId(payload.trip_id),
            vehicle_id: VehicleId(payload.vehicle_id),
            timestamp: DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
                .unwrap_or_default(),
            point: Point::new(point.longitude, point.latitude),
        }
    }
}

impl Payload {
    pub fn as_event(&self) -> RawEvent {
        RawEvent {
            vehicle_id: self.vehicle_id,
            point: self.point,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawEvent {
    pub vehicle_id: VehicleId,
    pub point: Point,

    /// When the observation was made. Serialized as microseconds since the
    /// Unix epoch on the wire.
    #[serde(with = "chrono::serde::ts_microseconds")]
    pub timestamp: DateTime<Utc>,
}

impl Storable for RawEvent {
    type ShardId = Geohash;
    type Key = VehicleId;

    fn shard_id(&self) -> Self::ShardId {
        GeohashStrategy::with_precision(4).locate(self.point)
    }

    fn key(&self) -> Self::Key {
        self.vehicle_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ingest surface must survive its protobuf round trip exactly:
    /// a foreign producer encoding `routers.realtime.v1.Payload` and this
    /// crate must agree on every field.
    #[test]
    fn payload_round_trips_over_the_wire() {
        let payload = Payload {
            trip_id: TripId(0xdead),
            vehicle_id: VehicleId(0xbeef),
            timestamp: DateTime::from_timestamp_micros(1_775_000_000_123_456).unwrap(),
            point: Point::new(150.871294, -33.938879),
        };

        let bytes = payload.encode().expect("payload must encode");
        let decoded = Payload::decode(&bytes).expect("payload must decode");

        assert_eq!(decoded.trip_id, payload.trip_id);
        assert_eq!(decoded.vehicle_id, payload.vehicle_id);
        assert_eq!(decoded.timestamp, payload.timestamp);
        assert_eq!(decoded.point, payload.point);
    }
}
