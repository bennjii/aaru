use routers::Graph;

use crate::model::EdgeMetadata;
use codec::osm::OsmEntryId;
use codec::osm::element::Tags;
use codec::{Entry, Metadata};
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

impl RouteService<OsmEntryId, Tags> {
    pub fn from_file(file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService {
            graph,
            pick: Box::new(|_| EdgeMetadata::default()),
        })
    }
}
