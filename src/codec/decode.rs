//! Decode the .osm.pbf into a graphical entity.

use std::fs::File;

/// `OSM`
/// This parses osm protobuf files.
///
/// The `.osm.pbf` format is described by the OSM proto. See the wiki.
/// https://wiki.openstreetmap.org/wiki/PBF_Format
///
///
struct Decode {}

impl Decode {
    fn from_file(file: impl Into<File>) {
        todo!();
    }
}