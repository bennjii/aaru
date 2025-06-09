use codec::osm::meta::OsmEdgeMetadata;
use codec::osm::{OsmEntryId, RuntimeTraversalConfig};
use codec::{Entry, Metadata};
use routers::Graph;

use std::marker::PhantomData;
use std::path::PathBuf;

pub mod matcher;
pub mod optimise;
pub mod proximity;

pub trait RuntimeContext: Send + Sync {
    // TODO: Remove
    fn new() -> Self;
}

// Implementations
impl RuntimeContext for RuntimeTraversalConfig {
    fn new() -> Self {
        RuntimeTraversalConfig::default()
    }
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

impl RouteService<OsmEntryId, OsmEdgeMetadata, RuntimeTraversalConfig> {
    pub fn from_file(file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService {
            graph,
            phantom: PhantomData,
        })
    }
}
