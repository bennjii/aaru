//! WKB geometry decoding for the GeoParquet reader.
//!
//! Overture stores geometry as a WKB binary column. Connectors are points; we
//! extract their coordinate via `geo_traits` without pulling in the
//! `geo-types` conversion feature.

use geo::{Coord, LineString, Point};
use geo_traits::{CoordTrait, GeometryTrait, GeometryType, LineStringTrait, PointTrait};
use wkb::reader::read_wkb;

use crate::overture::error::OvertureError;

/// Decodes a WKB `Point` (a connector geometry) into a [`geo::Point`].
pub fn wkb_to_point(bytes: &[u8]) -> Result<Point, OvertureError> {
    let wkb = read_wkb(bytes).map_err(|e| OvertureError::Decode(format!("wkb: {e:?}")))?;

    match wkb.as_type() {
        GeometryType::Point(point) => {
            let coord = point
                .coord()
                .ok_or_else(|| OvertureError::Decode("empty connector point".into()))?;
            Ok(Point::new(coord.x(), coord.y()))
        }
        _ => Err(OvertureError::Decode(
            "expected a Point geometry for a connector".into(),
        )),
    }
}

/// Decodes a WKB `LineString` (a segment geometry) into a [`geo::LineString`].
pub fn wkb_to_linestring(bytes: &[u8]) -> Result<LineString, OvertureError> {
    let wkb = read_wkb(bytes).map_err(|e| OvertureError::Decode(format!("wkb: {e:?}")))?;

    match wkb.as_type() {
        GeometryType::LineString(line) => Ok(LineString::new(
            line.coords()
                .map(|c| Coord { x: c.x(), y: c.y() })
                .collect(),
        )),
        _ => Err(OvertureError::Decode(
            "expected a LineString geometry for a segment".into(),
        )),
    }
}
