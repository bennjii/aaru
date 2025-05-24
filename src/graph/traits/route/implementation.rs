use crate::graph::Scan;
use crate::graph::traits::route::definition::Route;
use crate::graph::{Graph, Weight};

use codec::{Entry, Node};

use geo::Point;
use log::debug;
use petgraph::visit::EdgeRef;

impl<E> Route<E> for Graph<E>
where
    E: Entry,
{
    fn route_nodes(&self, start_node: E, finish_node: E) -> Option<(Weight, Vec<Node<E>>)> {
        debug!("Routing {start_node:?} -> {finish_node:?}");

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node,
            |finish| finish == finish_node,
            |e| e.weight().0,
            |_| 0 as Weight,
        )?;

        let route = path
            .iter()
            .filter_map(|v| self.hash.get(v).copied())
            .collect();

        Some((score, route))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    fn route_points(&self, start: Point, finish: Point) -> Option<(Weight, Vec<Node<E>>)> {
        let start_node = self.nearest_node(start)?;
        let finish_node = self.nearest_node(finish)?;
        self.route_nodes(start_node.id, finish_node.id)
    }
}
