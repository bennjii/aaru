//! The Overture routing network.
//!
//! [`OvertureNetwork`] is a directed graph over interned connector ids,
//! carrying per-segment metadata, spatial node/edge indices, and a versioned
//! `.rt` on-disk format. It implements the `routers_network` traits
//! (`DataPlane`, `Discovery`, `Scan`, `Route`), so any consumer generic over
//! `N: Network` accepts it.

use core::fmt::Debug;
use core::hash::BuildHasherDefault;

use geo::{Point, Rect};
use log::debug;
use petgraph::prelude::DiGraphMap;
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use web_time::Instant;

use routers_network::edge::Weight;
use routers_network::network::GraphEdge;
use routers_network::{
    DirectionAwareEdgeId, Discovery, Edge, Node, Route, RowIndex, Scan, envelope_of,
};

use crate::overture::element::{Connector, Segment};
use crate::overture::id::OvertureEntryId;
use crate::overture::meta::OvertureEdgeMetadata;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(all(feature = "overture", not(target_arch = "wasm32")))]
use crate::overture::error::OvertureError;

/// The directed graph backing an [`OvertureNetwork`].
pub type GraphStructure<E> =
    DiGraphMap<E, (Weight, DirectionAwareEdgeId<E>), BuildHasherDefault<FxHasher>>;

/// Magic header stapled at the start of every Overture `.rt` file.
const SAVE_MAGIC: &[u8; 4] = b"OVMN";

// Prevent files from being used across build revisions. Shares the crate-wide
// format hash emitted by `build.rs`.
include!(concat!(env!("OUT_DIR"), "/format_hash.rs"));
const SAVE_VERSION: u64 = FORMAT_HASH;

#[derive(Serialize, Deserialize)]
pub struct OvertureNetwork {
    pub graph: GraphStructure<OvertureEntryId>,
    pub hash: FxHashMap<OvertureEntryId, Node<OvertureEntryId>>,
    pub meta: FxHashMap<OvertureEntryId, OvertureEdgeMetadata>,

    #[serde(skip)]
    pub index: RowIndex<OvertureEntryId>,
    /// Edge rows are fully-interned (`Node`-carrying) so `edges_in_box` needs
    /// no hash lookups.
    #[serde(skip)]
    pub index_edge: RowIndex<Edge<Node<OvertureEntryId>>>,
}

impl OvertureNetwork {
    /// Build a network directly from parsed connectors and segments. Pure and
    /// filesystem-free — the unit-testable core of ingestion.
    pub fn from_elements(connectors: Vec<Connector>, segments: Vec<Segment>) -> Self {
        crate::overture::builder::build(connectors, segments)
    }

    /// Decode a previously-encoded `OvertureNetwork` from bytes and rebuild its
    /// spatial indices. Filesystem-free; suitable for WASM.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        const HEADER_LEN: usize = SAVE_MAGIC.len() + 8;

        if bytes.len() < HEADER_LEN || &bytes[..SAVE_MAGIC.len()] != SAVE_MAGIC {
            return Err("Header bytes are missing, try rebuilding the cache.".to_string());
        }

        let version = u64::from_le_bytes(
            bytes[SAVE_MAGIC.len()..HEADER_LEN]
                .try_into()
                .expect("8 bytes"),
        );

        if version != SAVE_VERSION {
            return Err(format!(
                "Header expects {SAVE_VERSION:016x}, got format hash {version:016x}, rebuild the cache."
            ));
        }

        let deserialise_start = Instant::now();
        let mut net: Self =
            postcard::from_bytes(&bytes[HEADER_LEN..]).map_err(|v| v.to_string())?;

        let deserialise = deserialise_start.elapsed();
        let rebuild_start = Instant::now();
        net.rebuild_indices();

        debug!(
            "OvertureNetwork::from_bytes: {} bytes, deserialised in {:?}, rebuilt indices in {:?}",
            bytes.len(),
            deserialise,
            rebuild_start.elapsed()
        );

        Ok(net)
    }

    /// Encode `self` into a `Vec<u8>` with the format header prepended.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let payload: Vec<u8> =
            postcard::to_allocvec(self).map_err(|e| format!("failed to serialise value: {e}"))?;
        let mut out = Vec::with_capacity(SAVE_MAGIC.len() + 8 + payload.len());

        out.extend_from_slice(SAVE_MAGIC);
        out.extend_from_slice(&SAVE_VERSION.to_le_bytes());
        out.extend_from_slice(&payload);

        Ok(out)
    }

    /// Read a directory (or single file) of Overture transportation GeoParquet
    /// and assemble a network. Not available on WASM; requires the `overture`
    /// feature (Arrow/Parquet reader stack).
    #[cfg(all(feature = "overture", not(target_arch = "wasm32")))]
    pub fn from_geoparquet(path: &Path) -> Result<Self, OvertureError> {
        let start = Instant::now();
        let transportation = crate::overture::reader::read_transportation(path)?;
        debug!(
            "OvertureNetwork::from_geoparquet read {} connectors, {} segments in {:?}",
            transportation.connectors.len(),
            transportation.segments.len(),
            start.elapsed()
        );
        Ok(Self::from_elements(
            transportation.connectors,
            transportation.segments,
        ))
    }

    /// Build from GeoParquet or, if a cache exists, load it; write the cache on
    /// a cold build.
    #[cfg(all(feature = "overture", not(target_arch = "wasm32")))]
    pub fn from_geoparquet_and_save(
        source: &Path,
        saved_path: &PathBuf,
    ) -> Result<Self, String> {
        if saved_path.exists() {
            match OvertureNetwork::from_saved(saved_path) {
                Ok(g) => return Ok(g),
                Err(e) => log::warn!(
                    "OvertureNetwork cache at `{}` is unusable ({e}); rebuilding from GeoParquet",
                    saved_path.display()
                ),
            }
        }
        let graph = OvertureNetwork::from_geoparquet(source).map_err(|e| e.to_string())?;
        graph.save_to_file(saved_path)?;
        Ok(graph)
    }

    /// Read a saved `.rt` from disk. Not available on WASM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_saved(filename: &PathBuf) -> Result<Self, String> {
        let bytes = std::fs::read(filename).map_err(|v| v.to_string())?;
        Self::from_bytes(&bytes).map_err(|e| format!("cache file `{}`: {e}", filename.display()))
    }

    /// Persist this network to disk. Not available on WASM.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let bytes = self.to_bytes()?;
        let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        debug!(
            "OvertureNetwork::save_to_file wrote {} bytes (incl. 12-byte header, format {:016x}) to {}",
            bytes.len(),
            SAVE_VERSION,
            path.display()
        );
        Ok(())
    }

    /// Rebuilds the node and edge spatial indices from `hash` and `graph`.
    /// Safe to call at any time; run after loading a serialised network.
    pub fn rebuild_indices(&mut self) {
        let nodes: Vec<OvertureEntryId> = self.hash.keys().copied().collect();
        let edges: Vec<Edge<Node<OvertureEntryId>>> = self
            .graph
            .all_edges()
            .filter_map(|(s, t, &(weight, id))| {
                let source = *self.hash.get(&s)?;
                let target = *self.hash.get(&t)?;
                Some(Edge {
                    source,
                    target,
                    id: DirectionAwareEdgeId::new(Node::new(Point::new(0., 0.), id.index()))
                        .with_direction(id.direction()),
                    weight,
                })
            })
            .collect();

        let hash = &self.hash;
        let (node_index, edge_index) = rayon::join(
            || {
                RowIndex::build(nodes, |id| {
                    let p = hash[id].position;
                    (p, p)
                })
            },
            || RowIndex::build(edges, |e| envelope_of(e.source.position, e.target.position)),
        );
        self.index = node_index;
        self.index_edge = edge_index;
    }

    pub fn num_nodes(&self) -> usize {
        self.graph.node_count()
    }
}

impl Default for OvertureNetwork {
    fn default() -> Self {
        Self {
            graph: GraphStructure::new(),
            hash: FxHashMap::default(),
            meta: FxHashMap::default(),
            index: RowIndex::default(),
            index_edge: RowIndex::default(),
        }
    }
}

impl Discovery for OvertureNetwork {
    fn edges_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = Edge<Node<OvertureEntryId>>> + Send + 'a> {
        Box::new(self.index_edge.search(bounds).copied())
    }

    fn nodes_in_box<'a>(
        &'a self,
        bounds: Rect<f64>,
    ) -> Box<dyn Iterator<Item = &'a Node<OvertureEntryId>> + Send + 'a> {
        Box::new(self.index.search(bounds).filter_map(|id| self.hash.get(id)))
    }

    fn node(&self, id: &OvertureEntryId) -> Option<&Node<OvertureEntryId>> {
        self.hash.get(id)
    }

    fn edge(
        &self,
        &source: &OvertureEntryId,
        &target: &OvertureEntryId,
    ) -> Option<Edge<OvertureEntryId>> {
        self.graph
            .edge_weight(source, target)
            .map(|&(weight, id)| Edge {
                source,
                target,
                weight,
                id,
            })
    }
}

impl Scan for OvertureNetwork {
    fn nearest_node<'a>(&'a self, point: &Point) -> Option<&'a Node<OvertureEntryId>> {
        self.index.nearest(point).and_then(|id| self.hash.get(id))
    }
}

impl Route for OvertureNetwork {
    fn route_nodes(
        &self,
        start_node: OvertureEntryId,
        finish_node: OvertureEntryId,
    ) -> Option<(Weight, Vec<Node<OvertureEntryId>>)> {
        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node,
            |finish| finish == finish_node,
            |(_, _, w)| w.0,
            |_| 0 as Weight,
        )?;

        let route = path
            .iter()
            .filter_map(|v| self.hash.get(v).copied())
            .collect();

        Some((score, route))
    }
}

impl Debug for OvertureNetwork {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("overture maps : network")
    }
}

impl routers_network::DataPlane for OvertureNetwork {
    type Entry = OvertureEntryId;
    type Runtime = <OvertureEdgeMetadata as routers_network::Metadata>::Runtime;
    type Meta = OvertureEdgeMetadata;

    fn metadata(&self, id: &OvertureEntryId) -> Option<&OvertureEdgeMetadata> {
        self.meta.get(id)
    }

    fn point(&self, id: &OvertureEntryId) -> Option<Point> {
        self.hash.get(id).map(|v| v.position)
    }

    fn edges_into<'a>(
        &'a self,
        id: OvertureEntryId,
    ) -> Box<dyn Iterator<Item = GraphEdge<OvertureEntryId>> + 'a> {
        Box::new(
            self.graph
                .edges_directed(id, petgraph::Direction::Incoming)
                .map(|(src, dst, &data)| (src, dst, data)),
        )
    }

    fn edges_outof<'a>(
        &'a self,
        id: OvertureEntryId,
    ) -> Box<dyn Iterator<Item = GraphEdge<OvertureEntryId>> + 'a> {
        Box::new(
            self.graph
                .edges_directed(id, petgraph::Direction::Outgoing)
                .map(|(src, dst, &data)| (src, dst, data)),
        )
    }

    fn fatten(
        &self,
        Edge {
            source,
            target,
            weight,
            id,
        }: &Edge<OvertureEntryId>,
    ) -> Option<Edge<Node<OvertureEntryId>>> {
        Some(Edge {
            source: *self.hash.get(source)?,
            target: *self.hash.get(target)?,
            id: DirectionAwareEdgeId::new(Node::new(Point::new(0., 0.), id.index())),
            weight: *weight,
        })
    }
}
