use routers::Graph;

use codec::Entry;
use codec::osm::OsmEntryId;
use std::path::PathBuf;

pub mod matcher;
pub mod optimise;
pub mod proximity;

#[derive(Debug)]
pub struct RouteService<E>
where
    E: Entry,
{
    pub graph: Graph<E>,
}

impl RouteService<OsmEntryId> {
    pub fn from_file(
        file: PathBuf,
    ) -> Result<RouteService<OsmEntryId>, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService { graph })
    }
}
