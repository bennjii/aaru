//! Defines internal translations and relevant utilities
//! in order to make the model useful as an SDK.

use crate::r#match::{MatchRequest, MatchResponse, SnapRequest};
use crate::model::{Coordinate, EdgeIdentifier, EdgeMetadata, NodeIdentifier};

use codec::primitive::GenericMetadata;
use codec::{Entry, Node};
use geo::{Coord, LineString, coord};
use std::fmt::Error as StdError;
use std::ops::Deref;

impl From<Coord> for Coordinate {
    fn from(value: Coord) -> Self {
        Coordinate {
            longitude: value.x,
            latitude: value.y,
        }
    }
}

pub struct Coordinates(Vec<Coordinate>);

impl From<Coordinates> for LineString {
    fn from(val: Coordinates) -> Self {
        val.iter()
            .map(|c| coord! { x: c.longitude, y: c.latitude })
            .collect::<LineString>()
    }
}

impl From<LineString> for Coordinates {
    fn from(value: LineString) -> Self {
        Coordinates(value.into_iter().map(|point| point.into()).collect())
    }
}

impl Coordinates {
    pub fn linestring(self) -> LineString {
        LineString::from(self)
    }
}

impl Deref for Coordinates {
    type Target = Vec<Coordinate>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MatchResponse {
    pub fn interpolated(&self) -> Option<Coordinates> {
        let path = self
            .matches
            .first()?
            .interpolated
            .iter()
            .filter_map(|element| element.coordinate)
            .collect::<Vec<_>>();

        Some(Coordinates(path))
    }

    pub fn discretized(&self) -> Option<Coordinates> {
        let path = self
            .matches
            .first()?
            .discretized
            .iter()
            .filter_map(|element| element.coordinate)
            .collect::<Vec<_>>();

        Some(Coordinates(path))
    }
}

impl From<GenericMetadata> for EdgeMetadata {
    fn from(val: GenericMetadata) -> Self {
        // TODO: Fill all the information out here...
        EdgeMetadata {
            lane_count: val.lane_count.map(|v| v.get() as u32),
            speed_limit: val.speed_limit.map(|v| v.get() as u32),
            names: vec![],
        }
    }
}

impl From<i64> for EdgeIdentifier {
    fn from(val: i64) -> Self {
        EdgeIdentifier { id: val }
    }
}

impl<E> From<Node<E>> for NodeIdentifier
where
    E: Entry,
{
    fn from(Node { id, position }: Node<E>) -> Self {
        NodeIdentifier {
            id: id.identifier(),
            coordinate: Some(<geo::Point as Into<Coord>>::into(position).into()),
        }
    }
}

impl MatchRequest {
    pub fn linestring(self) -> LineString {
        Into::<LineString>::into(Coordinates(self.data))
    }
}

impl SnapRequest {
    pub fn linestring(self) -> LineString {
        Into::<LineString>::into(Coordinates(self.data))
    }
}

impl TryFrom<MatchResponse> for LineString {
    type Error = StdError;

    fn try_from(value: MatchResponse) -> Result<Self, Self::Error> {
        let linestring = value.discretized().ok_or(StdError)?.into();

        Ok(linestring)
    }
}
