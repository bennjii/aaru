//! Serde row structs mirroring the Overture GeoParquet columns.
//!
//! These are deserialized straight from an Arrow `RecordBatch` via
//! `serde_arrow`, which handles Overture's nested struct/list columns. Only the
//! fields relevant to routing are modelled; the geometry column is read
//! separately (segments do not need it — node positions come from connectors).

use serde::Deserialize;

/// A row of the `type=segment` parquet (geometry column excluded).
#[derive(Debug, Clone, Deserialize)]
pub struct SegmentRow {
    pub id: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub subclass: Option<String>,
    #[serde(default)]
    pub connectors: Option<Vec<ConnectorRefRow>>,
    #[serde(default)]
    pub speed_limits: Option<Vec<SpeedLimitRow>>,
    #[serde(default)]
    pub access_restrictions: Option<Vec<AccessRestrictionRow>>,
}

/// An entry of a segment's `connectors` array: `{connector_id, at}`.
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectorRefRow {
    pub connector_id: String,
    #[serde(default)]
    pub at: Option<f64>,
}

/// A `{value, unit}` speed struct.
#[derive(Debug, Clone, Deserialize)]
pub struct SpeedRow {
    #[serde(default)]
    pub value: Option<i64>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// A vehicle-dimension condition (`when.vehicle` array entry), e.g.
/// `height greater_than 4.2 m`.
#[derive(Debug, Clone, Deserialize)]
pub struct VehicleConditionRow {
    #[serde(default)]
    pub dimension: Option<String>,
    #[serde(default)]
    pub comparison: Option<String>,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
}

/// The shared `when` conditional scope.
///
/// `heading` and `mode` are evaluated; the remaining fields mark a rule as
/// conditional so it is never applied as a blanket rule.
#[derive(Debug, Clone, Deserialize)]
pub struct WhenRow {
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub mode: Option<Vec<String>>,
    #[serde(default)]
    pub during: Option<String>,
    #[serde(default)]
    pub using: Option<Vec<String>>,
    #[serde(default)]
    pub recognized: Option<Vec<String>>,
    #[serde(default)]
    pub vehicle: Option<Vec<VehicleConditionRow>>,
}

impl WhenRow {
    /// Whether this scope carries conditions beyond `heading`/`mode` (time,
    /// purpose, recognised status, vehicle dimensions).
    pub fn is_conditional(&self) -> bool {
        self.during.is_some()
            || self.using.as_ref().is_some_and(|v| !v.is_empty())
            || self.recognized.as_ref().is_some_and(|v| !v.is_empty())
            || self.vehicle.as_ref().is_some_and(|v| !v.is_empty())
    }
}

/// An entry of a segment's `speed_limits` array.
#[derive(Debug, Clone, Deserialize)]
pub struct SpeedLimitRow {
    #[serde(default)]
    pub min_speed: Option<SpeedRow>,
    #[serde(default)]
    pub max_speed: Option<SpeedRow>,
    #[serde(default)]
    pub when: Option<WhenRow>,
}

/// An entry of a segment's `access_restrictions` array.
#[derive(Debug, Clone, Deserialize)]
pub struct AccessRestrictionRow {
    #[serde(default)]
    pub access_type: Option<String>,
    #[serde(default)]
    pub when: Option<WhenRow>,
}

/// Column names of [`SegmentRow`], used to project a record batch down to just
/// the fields we deserialize (dropping geometry/bbox/sources/etc.).
pub const SEGMENT_COLUMNS: &[&str] = &[
    "id",
    "subtype",
    "class",
    "subclass",
    "connectors",
    "speed_limits",
    "access_restrictions",
];
