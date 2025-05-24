//! Defines internal translations and relevant utilities
//! in order to make the model useful as an SDK.

use crate::r#match::MatchResponse;
use crate::model::Coordinate;

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
        Some(Coordinates(self.matches.first()?.interpolated.clone()))
    }

    pub fn snapped(&self) -> Option<Coordinates> {
        Some(Coordinates(self.matches.first()?.snapped_shape.clone()))
    }
}

impl TryFrom<MatchResponse> for LineString {
    type Error = StdError;

    fn try_from(value: MatchResponse) -> Result<Self, Self::Error> {
        let route = value.matches.first().ok_or(StdError)?;

        let linestring = Coordinates(route.snapped_shape.clone()).into();
        Ok(linestring)
    }
}
