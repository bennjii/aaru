pub mod r#match;
pub mod proximity;
pub mod route;

pub use r#match::Match;
pub use proximity::Scan;
pub use route::Route;

#[cfg(test)]
mod util {
    use crate::graph::Graph;
    use codec::osm::OsmEntryId;
    use routers_fixtures::fixture_path;

    use std::error::Error;
    use std::path::Path;
    use std::time::Instant;

    pub(crate) fn init_graph(file: &str) -> Result<Graph<OsmEntryId>, Box<dyn Error>> {
        let time = Instant::now();

        let fixture = fixture_path(file);
        let path = Path::new(&fixture);
        let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

        println!("Graph Init Took: {:?}", time.elapsed());
        Ok(graph)
    }
}
