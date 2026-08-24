//! End-to-end GeoParquet reader test.
//!
//! Generates a tiny connector/segment parquet pair at runtime (no committed
//! binary fixture), then reads it back through [`read_transportation`] and
//! [`OvertureNetwork::from_geoparquet`], exercising the full parquet →
//! serde_arrow → wkb → graph pipeline.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::{BinaryArray, StringArray};
use arrow::datatypes::{DataType, Field, FieldRef, Schema};
use arrow::record_batch::RecordBatch;
use geo::point;
use parquet::arrow::ArrowWriter;
use routers_network::{Route, Scan};
use serde::Serialize;
use serde_arrow::schema::{SchemaLike, TracingOptions};

use super::read_transportation;
use crate::overture::graph::OvertureNetwork;

// --- Serializable mirrors of the segment schema (write side) --------------

#[derive(Serialize)]
struct SegOut {
    id: String,
    subtype: Option<String>,
    class: Option<String>,
    subclass: Option<String>,
    connectors: Vec<ConnOut>,
    speed_limits: Vec<SpeedLimitOut>,
    access_restrictions: Vec<AccessOut>,
}

#[derive(Serialize)]
struct ConnOut {
    connector_id: String,
    at: f64,
}

#[derive(Serialize)]
struct SpeedOut {
    value: i64,
    unit: String,
}

#[derive(Serialize)]
struct WhenOut {
    heading: Option<String>,
    mode: Option<Vec<String>>,
}

#[derive(Serialize)]
struct SpeedLimitOut {
    min_speed: Option<SpeedOut>,
    max_speed: Option<SpeedOut>,
    when: Option<WhenOut>,
}

#[derive(Serialize)]
struct AccessOut {
    access_type: Option<String>,
    when: Option<WhenOut>,
}

/// Little-endian WKB encoding of a 2D point.
fn wkb_point(x: f64, y: f64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(21);
    buf.push(1u8); // little-endian byte order
    buf.extend_from_slice(&1u32.to_le_bytes()); // geometry type 1 = Point
    buf.extend_from_slice(&x.to_le_bytes());
    buf.extend_from_slice(&y.to_le_bytes());
    buf
}

fn write_parquet(path: &Path, batch: &RecordBatch) {
    let file = File::create(path).expect("create parquet");
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None).expect("writer");
    writer.write(batch).expect("write batch");
    writer.close().expect("close writer");
}

fn write_connectors(path: &Path) {
    let ids = StringArray::from(vec!["c1", "c2", "c3"]);
    let geoms = BinaryArray::from(
        [
            wkb_point(0.0, 0.0),
            wkb_point(1.0, 0.0),
            wkb_point(2.0, 0.0),
        ]
        .iter()
        .map(|v| v.as_slice())
        .collect::<Vec<_>>(),
    );

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("geometry", DataType::Binary, false),
    ]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(geoms)])
        .expect("connector batch");
    write_parquet(path, &batch);
}

fn segment_samples() -> Vec<SegOut> {
    let when = || WhenOut {
        heading: Some("forward".into()),
        mode: Some(vec!["car".into()]),
    };
    vec![
        SegOut {
            id: "s100".into(),
            subtype: Some("road".into()),
            class: Some("primary".into()),
            subclass: None,
            connectors: vec![
                ConnOut { connector_id: "c1".into(), at: 0.0 },
                ConnOut { connector_id: "c2".into(), at: 0.5 },
                ConnOut { connector_id: "c3".into(), at: 1.0 },
            ],
            speed_limits: vec![SpeedLimitOut {
                min_speed: Some(SpeedOut { value: 30, unit: "km/h".into() }),
                max_speed: Some(SpeedOut { value: 50, unit: "km/h".into() }),
                when: Some(when()),
            }],
            access_restrictions: vec![],
        },
        SegOut {
            id: "s200".into(),
            subtype: Some("road".into()),
            class: Some("primary".into()),
            subclass: Some("link".into()),
            connectors: vec![
                ConnOut { connector_id: "c2".into(), at: 0.0 },
                ConnOut { connector_id: "c3".into(), at: 1.0 },
            ],
            speed_limits: vec![],
            access_restrictions: vec![AccessOut {
                access_type: Some("denied".into()),
                when: Some(when()),
            }],
        },
        // A non-road segment that must be ignored by the reader.
        SegOut {
            id: "w1".into(),
            subtype: Some("water".into()),
            class: None,
            subclass: None,
            connectors: vec![
                ConnOut { connector_id: "c1".into(), at: 0.0 },
                ConnOut { connector_id: "c3".into(), at: 1.0 },
            ],
            speed_limits: vec![],
            access_restrictions: vec![],
        },
    ]
}

fn write_segments(path: &Path) {
    let samples = segment_samples();
    let fields =
        Vec::<FieldRef>::from_samples(&samples, TracingOptions::default()).expect("trace schema");
    let batch = serde_arrow::to_record_batch(&fields, &samples).expect("segment batch");
    write_parquet(path, &batch);
}

/// A fresh temp directory laid out like an Overture partition. `name` keeps
/// concurrent tests from sharing (and clobbering) the same path.
fn fixture_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("routers_overture_reader_test_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("type=connector")).expect("mkdir connector");
    fs::create_dir_all(dir.join("type=segment")).expect("mkdir segment");
    write_connectors(&dir.join("type=connector/part-0.parquet"));
    write_segments(&dir.join("type=segment/part-0.parquet"));
    dir
}

#[test]
fn reads_transportation_geoparquet_into_elements() {
    let dir = fixture_dir("elements");
    let transportation = read_transportation(&dir).expect("read");

    assert_eq!(transportation.connectors.len(), 3, "three connectors decoded");
    // Two road segments kept; the water segment dropped.
    assert_eq!(transportation.segments.len(), 2, "only road segments retained");

    // Connector geometry decoded from WKB.
    let xs: Vec<f64> = transportation
        .connectors
        .iter()
        .map(|c| c.position.x())
        .collect();
    assert!(xs.contains(&0.0) && xs.contains(&2.0));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn builds_network_from_geoparquet() {
    let dir = fixture_dir("network");
    let net = OvertureNetwork::from_geoparquet(&dir).expect("build network");

    assert_eq!(net.num_nodes(), 3);
    // c1↔c2 and c2↔c3 → four directed edges.
    assert_eq!(net.graph.edge_count(), 4);

    // The chain is routable end to end.
    let route = net.route_points(&point! { x: 0.0, y: 0.0 }, &point! { x: 2.0, y: 0.0 });
    assert!(route.is_some(), "route across the segment chain");

    // The 50 km/h max speed survived parquet → serde_arrow → domain.
    let has_speed = net.meta.values().any(|m| {
        m.speed_limits
            .iter()
            .any(|s| s.max_speed.map(|v| v.in_kmh()) == Some(50))
    });
    assert!(has_speed, "parsed max speed limit present");

    // A nearest-node lookup confirms the spatial index is populated.
    assert!(net.nearest_node(&point! { x: 0.1, y: 0.0 }).is_some());

    let _ = fs::remove_dir_all(&dir);
}
