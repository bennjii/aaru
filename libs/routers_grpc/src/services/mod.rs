use routers::Graph;
use routers::transition::PredicateCache;

use codec::osm::element::variants::OsmEntryId;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod matcher;
pub mod optimise;
pub mod proximity;

#[derive(Debug)]
pub struct RouteService {
    pub graph: Graph<OsmEntryId>,
    lookup: Arc<Mutex<PredicateCache<OsmEntryId>>>,
}

impl RouteService {
    pub fn from_file(file: PathBuf) -> Result<RouteService, Box<dyn std::error::Error>> {
        let graph =
            Graph::new(file.as_os_str().to_ascii_lowercase()).map_err(|e| format!("{:?}", e))?;

        Ok(RouteService {
            graph,
            lookup: Arc::new(Mutex::new(PredicateCache::default())),
        })
    }
}
