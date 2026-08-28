//! Assembles connectors + segments into an [`OvertureNetwork`].
//!
//! Each connector becomes a graph node. Each navigable segment is split at
//! its sorted connector positions, and the linestring's interior vertices
//! between two connectors are materialised as synthetic nodes, so the edge
//! chain follows the road shape rather than cutting the corner. Pure (no
//! IO), so it is unit-testable with hand-built elements.

use geo::Point;
use routers_network::{DirectionAwareEdgeId, Metadata, Node};
use rustc_hash::FxHashMap;

use crate::overture::element::{Connector, Segment};
use crate::overture::graph::{GraphStructure, OvertureNetwork};
use crate::overture::id::OvertureEntryId;
use crate::overture::meta::OvertureEdgeMetadata;
use crate::overture::parsers::Heading;

/// Builds an [`OvertureNetwork`] from parsed elements.
///
/// Edge pairs referencing a connector absent from the connector set
/// (dangling references after a bbox clip) are dropped: every node in the
/// graph must carry a position, an invariant routing consumers rely on.
pub fn build(connectors: Vec<Connector>, segments: Vec<Segment>) -> OvertureNetwork {
    let mut hash: FxHashMap<OvertureEntryId, Node<OvertureEntryId>> = FxHashMap::default();
    for connector in &connectors {
        hash.insert(connector.id, Node::new(connector.position, connector.id));
    }

    // Synthetic (shape-vertex) node ids are allocated past every interned id
    // so they can never collide with a connector or segment.
    let mut next_synthetic = connectors
        .iter()
        .map(|c| c.id.identifier)
        .chain(segments.iter().map(|s| s.id.identifier))
        .max()
        .unwrap_or(-1)
        + 1;

    let mut graph = GraphStructure::new();
    let mut meta: FxHashMap<OvertureEntryId, OvertureEdgeMetadata> = FxHashMap::default();

    for segment in &segments {
        if !segment.navigable() {
            continue;
        }

        let weight = segment.weight();
        let forward = segment.open(Heading::Forward);
        let backward = segment.open(Heading::Backward);
        let direction_aware = DirectionAwareEdgeId::new(segment.id);

        meta.insert(segment.id, OvertureEdgeMetadata::pick(segment));

        for pair in segment.connectors.windows(2) {
            if let [a, b] = pair {
                // Both endpoints must resolve to a positioned connector.
                if !hash.contains_key(&a.id) || !hash.contains_key(&b.id) {
                    continue;
                }

                // The chain a → shape vertices → b.
                let chain: Vec<OvertureEntryId> = core::iter::once(a.id)
                    .chain(segment.interior_vertices(a.at, b.at).map(|coord| {
                        let id = OvertureEntryId::new(next_synthetic);
                        next_synthetic += 1;
                        hash.insert(id, Node::new(Point(coord), id));
                        id
                    }))
                    .chain(core::iter::once(b.id))
                    .collect();

                for hop in chain.windows(2) {
                    if let [s, t] = hop {
                        if forward {
                            graph.add_edge(*s, *t, (weight, direction_aware.forward()));
                        }
                        if backward {
                            graph.add_edge(*t, *s, (weight, direction_aware.backward()));
                        }
                    }
                }
            }
        }
    }

    // Keep only nodes that made it into the graph; isolated connectors would
    // otherwise bloat the hash and spatial index.
    let mut pruned: FxHashMap<OvertureEntryId, Node<OvertureEntryId>> = FxHashMap::default();
    for (id, node) in hash {
        if graph.contains_node(id) {
            pruned.insert(id, node);
        }
    }

    let mut network = OvertureNetwork {
        graph,
        hash: pruned,
        meta,
        index: Default::default(),
        index_edge: Default::default(),
    };
    network.rebuild_indices();
    network
}
