use codec::osm::meta::OsmEdgeMetadata;
use codec::osm::{OsmEntryId, TraversalConditions};
use codec::{Entry, Metadata};
use routers::Graph;
use std::marker::PhantomData;

use codec::osm::primitives::{Directionality, TransportMode};
use std::path::PathBuf;

pub mod matcher;
pub mod optimise;
pub mod proximity;

pub trait RuntimeContext: Send + Sync {
    fn new() -> Self;
}

pub struct RouteService<E, M, Ctx>
where
    E: Entry,
    M: Metadata,
    Ctx: RuntimeContext,
{
    pub graph: Graph<E, M>,

    phantom: PhantomData<Ctx>,
}

impl RouteService<OsmEntryId, OsmEdgeMetadata, TraversalConditions> {
    pub fn from_file(file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService {
            graph,
            phantom: PhantomData,
        })
    }
}

impl RuntimeContext for TraversalConditions {
    fn new() -> Self {
        TraversalConditions {
            directionality: Directionality::BothWays,
            transport_mode: TransportMode::Vehicle,
            lane: None,
        }
    }
}
