//! Component bindings: generate the Rust types/traits from `wit/world.wit` and
//! map the exported `router` interface onto the pure [`engine`](crate::engine)
//! core. Compiled only for wasm — the component export glue is meaningless on
//! the host.

use core::cell::RefCell;

use geo::{Coord, Distance, Geodesic};
use routers_codec::osm::{OsmEdgeMetadata, OsmEntryId};
use routers_codec::primitive::transport::{TransportMode, TruckCosting, VehicleCosting};
use routers_network::{Edge, Entry, Node};
use routers_transition::candidate::{Path, PathElement};
use routers_transition::weigh::SolverVariant;

use crate::engine::{Engine as Core, Failure, Options};

wit_bindgen::generate!({
    world: "routers",
});

use exports::routers::routing::router as wit;

struct Component;

impl wit::Guest for Component {
    type Engine = EngineResource;
}

/// Backing state for the exported `engine` resource. Resource methods take
/// `&self`, but load/unload mutate; `RefCell` gives interior mutability
/// (a wasm component is single-threaded, so this never contends).
struct EngineResource {
    core: RefCell<Core>,
}

impl wit::GuestEngine for EngineResource {
    fn new() -> Self {
        EngineResource {
            core: RefCell::new(Core::new()),
        }
    }

    fn load_shard(&self, id: String, bytes: Vec<u8>) -> Result<(), wit::Error> {
        self.core
            .borrow_mut()
            .load_shard(id, &bytes)
            .map_err(wit::Error::InvalidNetwork)
    }

    fn unload_shard(&self, id: String) {
        self.core.borrow_mut().unload_shard(&id);
    }

    fn loaded_shards(&self) -> Vec<String> {
        self.core.borrow().loaded()
    }

    fn shard_of(point: wit::Coordinate) -> String {
        Core::shard_of(to_coord(&point))
    }

    fn shard_neighbours(id: String) -> Result<Vec<String>, wit::Error> {
        Core::shard_neighbours(&id).map_err(wit::Error::InvalidShard)
    }

    fn match_(
        &self,
        trace: Vec<wit::Coordinate>,
        options: wit::MatchOptions,
    ) -> Result<wit::MatchedRoute, wit::Error> {
        let coords: Vec<Coord> = trace.iter().map(to_coord).collect();

        let opts = Options {
            search_distance: options.search_distance,
            reach_distance: options.reach_distance,
            solver: solver_of(options.optimise_for),
            transport: options.transport.map(transport_of),
        };

        match self.core.borrow().match_trace(&coords, opts) {
            Ok(routed) => Ok(wit::MatchedRoute {
                discretized: elements(&routed.discretized),
                interpolated: elements(&routed.interpolated),
                cost: 0,
            }),
            Err(Failure::Unanchored(i)) => Err(wit::Error::Unanchored(i)),
            Err(Failure::Disconnected(i)) => Err(wit::Error::Disconnected(i)),
            Err(Failure::Solve(s)) => Err(wit::Error::SolveFailed(s)),
        }
    }

    fn route(
        &self,
        start: wit::Coordinate,
        end: wit::Coordinate,
    ) -> Result<wit::Route, wit::Error> {
        self.core
            .borrow()
            .route(to_coord(&start), to_coord(&end))
            .map(|(cost, shape)| wit::Route {
                shape: shape.iter().map(to_wit).collect(),
                cost,
            })
            .ok_or_else(|| wit::Error::NotFound("no route between the points".into()))
    }

    fn nearest(&self, point: wit::Coordinate) -> Result<wit::Coordinate, wit::Error> {
        self.core
            .borrow()
            .nearest(to_coord(&point))
            .map(|c| to_wit(&c))
            .ok_or_else(|| wit::Error::NotFound("no node near the point".into()))
    }

    fn snap(&self, point: wit::Coordinate, radius: f64) -> Result<wit::Coordinate, wit::Error> {
        self.core
            .borrow()
            .snap(to_coord(&point), radius)
            .map(|c| to_wit(&c))
            .ok_or_else(|| wit::Error::NotFound("no edge within the radius".into()))
    }

    fn nearest_edges(
        &self,
        point: wit::Coordinate,
        radius: f64,
    ) -> Result<Vec<wit::Edge>, wit::Error> {
        Ok(self
            .core
            .borrow()
            .nearest_edges(to_coord(&point), radius)
            .iter()
            .map(|edge| edge_of(edge, empty_metadata()))
            .collect())
    }
}

fn to_coord(c: &wit::Coordinate) -> Coord {
    Coord {
        x: c.longitude,
        y: c.latitude,
    }
}

fn to_wit(c: &Coord) -> wit::Coordinate {
    wit::Coordinate {
        latitude: c.y,
        longitude: c.x,
    }
}

fn solver_of(optimise: Option<wit::OptimiseFor>) -> SolverVariant {
    match optimise {
        Some(wit::OptimiseFor::Consistency) => SolverVariant::Selective,
        Some(wit::OptimiseFor::Parallelism) => SolverVariant::Precompute,
        _ => SolverVariant::Fastest,
    }
}

fn vehicle_of(v: wit::VehicleCosting) -> VehicleCosting {
    VehicleCosting {
        height: v.height,
        width: v.width,
    }
}

fn transport_of(transport: wit::Transport) -> TransportMode {
    match transport {
        wit::Transport::Car(v) => TransportMode::Car(Some(vehicle_of(v))),
        wit::Transport::Bus(v) => TransportMode::Bus(Some(vehicle_of(v))),
        wit::Transport::Truck(t) => TransportMode::Truck(Some(TruckCosting {
            vehicle_costing: VehicleCosting {
                height: t.height,
                width: t.width,
            },
            length: t.length,
            axle_load: t.axle_load,
            axle_count: t.axle_count as u8,
            hazmat_load: t.hazardous_load,
        })),
    }
}

fn node(n: &Node<OsmEntryId>) -> wit::NodeIdentifier {
    wit::NodeIdentifier {
        id: n.identifier(),
        coordinate: wit::Coordinate {
            latitude: n.position.y(),
            longitude: n.position.x(),
        },
    }
}

fn empty_metadata() -> wit::EdgeMetadata {
    wit::EdgeMetadata {
        lane_count: None,
        // Speed limits and names need the trip costing runtime; a follow-up.
        speed_limit: None,
        names: Vec::new(),
    }
}

fn edge_of(edge: &Edge<Node<OsmEntryId>>, metadata: wit::EdgeMetadata) -> wit::Edge {
    let (source, target) = (&edge.source, &edge.target);
    wit::Edge {
        id: edge.id().identifier(),
        source: node(source),
        target: node(target),
        length: Geodesic.distance(source.position, target.position),
        metadata,
    }
}

fn elements(path: &Path<OsmEntryId, OsmEdgeMetadata>) -> Vec<wit::RouteElement> {
    path.iter()
        .map(
            |PathElement {
                 point,
                 edge,
                 metadata,
             }| wit::RouteElement {
                coordinate: to_wit(point),
                edge: wit::RouteEdge {
                    edge: edge_of(
                        edge,
                        wit::EdgeMetadata {
                            lane_count: metadata.lane_count.map(|v| v.get() as u32),
                            ..empty_metadata()
                        },
                    ),
                    join_percent: 0,
                    depart_percent: 0,
                    routed_length: 0,
                },
            },
        )
        .collect()
}

export!(Component);
