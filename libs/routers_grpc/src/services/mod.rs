use codec::osm::OsmEntryId;
use codec::osm::meta::OsmEdgeMetadata;
use codec::{Entry, Metadata};
use routers::Graph;

use crate::model::EdgeMetadata;
use std::path::PathBuf;

pub mod matcher;
pub mod optimise;
pub mod proximity;

pub struct RouteService<E, M>
where
    E: Entry,
    M: Metadata,
{
    pub graph: Graph<E, M>,
    pub pick: Box<dyn Fn(M) -> EdgeMetadata + Sync + Send>,
}

impl RouteService<OsmEntryId, OsmEdgeMetadata> {
    pub fn from_file(file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService {
            graph,
            pick: Box::new(|_| EdgeMetadata::default()),
        })
    }
}
