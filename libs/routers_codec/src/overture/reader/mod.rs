//! GeoParquet reader for the Overture transportation theme.
//!
//! [`read_transportation`] takes a file or directory of Overture GeoParquet
//! and returns a [`Transportation`] set of parsed [`Connector`]s and
//! [`Segment`]s, ready for [`OvertureNetwork::from_elements`]. Connectors are
//! read first so their interned ids are stable before segments reference
//! them.
//!
//! The stack is pure Rust: `parquet` (row groups → Arrow batches),
//! `serde_arrow` (batch → [`rows`] structs), and `wkb` (connector point
//! geometry). Remote S3/Azure fetching is a documented follow-up.
//!
//! [`OvertureNetwork::from_elements`]: crate::overture::graph::OvertureNetwork::from_elements

pub mod geometry;
pub mod rows;

#[cfg(test)]
mod tests;

use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::array::{Array, BinaryArray, LargeBinaryArray, LargeStringArray, StringArray};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::overture::element::{Connector, Segment, SegmentConnector};
use crate::overture::error::OvertureError;
use crate::overture::id::Interner;
use crate::overture::parsers::{AccessRestriction, Heading, Speed, SpeedLimit, TravelMode};
use rows::{AccessRestrictionRow, SegmentRow, SpeedLimitRow, WhenRow};

/// The parsed contents of an Overture transportation extract.
pub struct Transportation {
    pub connectors: Vec<Connector>,
    pub segments: Vec<Segment>,
}

/// Reads all Overture transportation GeoParquet under `path` (a single file
/// or a directory searched recursively).
pub fn read_transportation(path: &Path) -> Result<Transportation, OvertureError> {
    let files = discover_parquet(path)?;
    if files.is_empty() {
        return Err(OvertureError::NoParquetFiles(path.display().to_string()));
    }

    // Classify each file by its schema, not its directory naming.
    let mut connector_files = Vec::new();
    let mut segment_files = Vec::new();
    for file in files {
        match classify(&file)? {
            Kind::Connector => connector_files.push(file),
            Kind::Segment => segment_files.push(file),
        }
    }

    let mut interner = Interner::new();

    let mut connectors = Vec::new();
    for file in &connector_files {
        read_connectors(file, &mut interner, &mut connectors)?;
    }

    let mut segments = Vec::new();
    for file in &segment_files {
        read_segments(file, &mut interner, &mut segments)?;
    }

    Ok(Transportation {
        connectors,
        segments,
    })
}

enum Kind {
    Connector,
    Segment,
}

/// A file is a segment file if it carries any segment-only column; otherwise
/// it is treated as a connector file.
fn classify(path: &Path) -> Result<Kind, OvertureError> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| OvertureError::Decode(format!("{}: {e}", path.display())))?;
    let has = |name: &str| builder.schema().fields().iter().any(|f| f.name() == name);
    if has("connectors") || has("class") || has("access_restrictions") || has("speed_limits") {
        Ok(Kind::Segment)
    } else {
        Ok(Kind::Connector)
    }
}

/// Recursively collects `*.parquet` files under `path`, or returns `[path]`
/// if it is itself a parquet file.
fn discover_parquet(path: &Path) -> Result<Vec<PathBuf>, OvertureError> {
    let mut out = Vec::new();
    if path.is_file() {
        if is_parquet(path) {
            out.push(path.to_path_buf());
        }
        return Ok(out);
    }

    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if is_parquet(&p) {
                out.push(p);
            }
        }
    }
    Ok(out)
}

fn is_parquet(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("parquet"))
}

fn batch_reader(
    path: &Path,
) -> Result<impl Iterator<Item = Result<RecordBatch, OvertureError>>, OvertureError> {
    let file = File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| OvertureError::Decode(format!("{}: {e}", path.display())))?
        .build()
        .map_err(|e| OvertureError::Decode(format!("{}: {e}", path.display())))?;
    Ok(reader.map(|b| b.map_err(|e| OvertureError::Decode(e.to_string()))))
}

fn read_connectors(
    path: &Path,
    interner: &mut Interner,
    out: &mut Vec<Connector>,
) -> Result<(), OvertureError> {
    for batch in batch_reader(path)? {
        let batch = batch?;
        let id_col = batch
            .column_by_name("id")
            .ok_or_else(|| OvertureError::Schema("connector: missing `id`".into()))?;
        let geom_col = batch
            .column_by_name("geometry")
            .ok_or_else(|| OvertureError::Schema("connector: missing `geometry`".into()))?;

        for i in 0..batch.num_rows() {
            let (Some(gers), Some(wkb_bytes)) = (str_at(id_col, i), bin_at(geom_col, i)) else {
                continue;
            };
            let position = geometry::wkb_to_point(wkb_bytes)?;
            let id = interner.intern(gers);
            out.push(Connector::new(id, position));
        }
    }
    Ok(())
}

fn read_segments(
    path: &Path,
    interner: &mut Interner,
    out: &mut Vec<Segment>,
) -> Result<(), OvertureError> {
    for batch in batch_reader(path)? {
        let batch = batch?;
        let projected = project(&batch, rows::SEGMENT_COLUMNS)?;
        let seg_rows: Vec<SegmentRow> = serde_arrow::from_record_batch(&projected)
            .map_err(|e| OvertureError::Decode(format!("segment rows: {e}")))?;

        // Geometry decodes separately from the serde rows (WKB binary); an
        // absent column yields chord-only segments.
        let geom_col = batch.column_by_name("geometry");

        for (i, row) in seg_rows.into_iter().enumerate() {
            let geometry = geom_col
                .and_then(|col| bin_at(col.as_ref(), i))
                .map(geometry::wkb_to_linestring)
                .transpose()?
                .unwrap_or_else(|| geo::LineString::new(Vec::new()));

            if let Some(segment) = row_to_segment(row, geometry, interner) {
                out.push(segment);
            }
        }
    }
    Ok(())
}

/// Projects a record batch down to the named columns, dropping the rest so
/// `serde_arrow` only sees fields we model.
fn project(batch: &RecordBatch, names: &[&str]) -> Result<RecordBatch, OvertureError> {
    let schema = batch.schema();
    let indices: Vec<usize> = schema
        .fields()
        .iter()
        .enumerate()
        .filter(|(_, f)| names.contains(&f.name().as_str()))
        .map(|(i, _)| i)
        .collect();
    batch
        .project(&indices)
        .map_err(|e| OvertureError::Decode(format!("projection: {e}")))
}

/// Converts a parsed segment row into a [`Segment`], interning ids. Returns
/// `None` for non-road segments — only `subtype: road` is routable.
fn row_to_segment(
    row: SegmentRow,
    geometry: geo::LineString,
    interner: &mut Interner,
) -> Option<Segment> {
    if row.subtype.as_deref() != Some("road") {
        return None;
    }

    let road_class = row.class.as_deref().and_then(|c| c.parse().ok());
    let is_link = row.subclass.as_deref() == Some("link");

    let connectors: Vec<SegmentConnector> = row
        .connectors
        .unwrap_or_default()
        .into_iter()
        .map(|c| SegmentConnector {
            id: interner.intern(&c.connector_id),
            at: c.at.unwrap_or(0.0),
        })
        .collect();

    let speed_limits = row
        .speed_limits
        .unwrap_or_default()
        .into_iter()
        .map(to_speed_limit)
        .collect();

    let access = row
        .access_restrictions
        .unwrap_or_default()
        .into_iter()
        .filter_map(to_access)
        .collect();

    let id = interner.intern(&row.id);
    Some(Segment::new(
        id,
        geometry,
        connectors,
        road_class,
        is_link,
        speed_limits,
        access,
    ))
}

fn to_speed_limit(row: SpeedLimitRow) -> SpeedLimit {
    let (heading, mode, conditional) = split_when(row.when);
    SpeedLimit {
        max_speed: row.max_speed.and_then(to_speed),
        min_speed: row.min_speed.and_then(to_speed),
        heading,
        mode,
        conditional,
    }
}

fn to_access(row: AccessRestrictionRow) -> Option<AccessRestriction> {
    let access_type = row.access_type.as_deref()?.parse().ok()?;
    let (heading, mode, conditional) = split_when(row.when);
    Some(AccessRestriction {
        access_type,
        heading,
        mode,
        conditional,
    })
}

fn to_speed(row: rows::SpeedRow) -> Option<Speed> {
    let value = row.value?;
    if value <= 0 {
        return None;
    }
    let unit = row.unit.as_deref()?.parse().ok()?;
    Some(Speed::new(value as u32, unit))
}

/// Splits a `when` scope into the evaluated parts (heading, modes) and a
/// conditional flag covering everything else.
fn split_when(when: Option<WhenRow>) -> (Option<Heading>, Vec<TravelMode>, bool) {
    match when {
        Some(w) => {
            let heading = w.heading.as_deref().and_then(|h| h.parse().ok());
            let mode = w
                .mode
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter_map(|m| m.parse().ok())
                .collect();
            (heading, mode, w.is_conditional())
        }
        None => (None, Vec::new(), false),
    }
}

/// Reads a UTF-8 value at index `i`, handling `Utf8` and `LargeUtf8`.
fn str_at(col: &dyn Array, i: usize) -> Option<&str> {
    if col.is_null(i) {
        return None;
    }
    if let Some(a) = col.as_any().downcast_ref::<StringArray>() {
        return Some(a.value(i));
    }
    if let Some(a) = col.as_any().downcast_ref::<LargeStringArray>() {
        return Some(a.value(i));
    }
    None
}

/// Reads a binary value at index `i`, handling `Binary` and `LargeBinary`.
fn bin_at(col: &dyn Array, i: usize) -> Option<&[u8]> {
    if col.is_null(i) {
        return None;
    }
    if let Some(a) = col.as_any().downcast_ref::<BinaryArray>() {
        return Some(a.value(i));
    }
    if let Some(a) = col.as_any().downcast_ref::<LargeBinaryArray>() {
        return Some(a.value(i));
    }
    None
}
