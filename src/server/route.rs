use geo::{coord, point, EuclideanDistance, GeometryCollection, HaversineDistance, Point};
use log::{debug, info};
use rstar::PointDistance;
use std::cmp::Ordering;
use std::path::Path;
use tonic::{Request, Response, Status};

use router_service::{MatchedRoute, RouteRequest, RouteResponse};

use crate::route::Graph;
use crate::server::route::router_service::{
    ClosestPointRequest, ClosestPointResponse, ClosestSnappedPointRequest,
    ClosestSnappedPointResponse, Coordinate, MapMatchRequest, MapMatchResponse,
};
use geo::LineString;
use router_service::router_service_server::RouterService;
#[cfg(feature = "tracing")]
use tracing::Level;
use wkt::ToWkt;

pub mod router_service {
    tonic::include_proto!("aaru.v1");

    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("aaru_descriptor");
}

#[derive(Debug)]
pub struct RouteService {
    graph: Graph,
}

impl RouteService {
    pub fn from_file(file: &str) -> crate::Result<RouteService> {
        let path = Path::new(file);
        let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

        Ok(RouteService { graph })
    }
}

#[tonic::async_trait]
impl RouterService for RouteService {
    #[cfg_attr(feature="tracing", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn route(
        &self,
        request: Request<RouteRequest>,
    ) -> Result<Response<RouteResponse>, Status> {
        let (_, _, routing) = request.into_parts();

        let start = routing
            .start
            .map_or(
                Err(Status::invalid_argument("Missing Start Coordinate")),
                |v| Ok(coord! { x: v.longitude, y: v.latitude }),
            )
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        let end = routing
            .end
            .map_or(
                Err(Status::invalid_argument("Missing End Coordinate")),
                |v| Ok(coord! { x: v.longitude, y: v.latitude }),
            )
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        self.graph.route(Point(start), Point(end)).map_or(
            Err(Status::internal("Could not route")),
            |(cost, route)| {
                let shape = route
                    .iter()
                    .map(|node| Coordinate {
                        latitude: node.position.y(),
                        longitude: node.position.x(),
                    })
                    .collect();

                Ok(Response::new(RouteResponse { cost, shape }))
            },
        )
    }

    #[cfg_attr(feature="tracing", tracing::instrument(skip_all, level = Level::INFO))]
    async fn map_match(
        &self,
        request: Request<MapMatchRequest>,
    ) -> Result<Response<MapMatchResponse>, Status> {
        let mapmatch = request.into_inner();
        let coordinates = mapmatch
            .data
            .iter()
            .map(|coord| coord! { x: coord.longitude, y: coord.latitude })
            .collect::<LineString>();

        let result = self
            .graph
            .map_match(coordinates, mapmatch.search_distance.unwrap_or(100.0))
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        // result.interpolated
        //     .iter()
        //     .map(|node| {
        //         node.node_idx.iter.map(|n| {
        //             let outgoing = self.graph.graph.edges_directed(n, petgraph::Direction::Outgoing);
        //         })
        //     })

        let snapped_shape = result
            .matched
            .iter()
            .map(|node| Coordinate {
                latitude: node.position.y(),
                longitude: node.position.x(),
            })
            .collect::<Vec<_>>();

        let interpolated = result
            .interpolated
            .coords()
            .into_iter()
            .map(|node| Coordinate {
                latitude: node.y,
                longitude: node.x,
            })
            .collect::<Vec<_>>();

        let matching = MatchedRoute {
            snapped_shape,
            interpolated,

            edges: vec![],
            label: "!".to_string(),
            cost: 0,
        };

        Ok(Response::new(MapMatchResponse {
            // TODO: Vector to allow trip-splitting in the future.
            matchings: vec![matching],
            // TODO: Aggregate all the errored trips.
            warnings: vec![],
        }))
    }

    #[cfg_attr(feature="tracing", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn closest_point(
        &self,
        request: Request<ClosestPointRequest>,
    ) -> Result<Response<ClosestPointResponse>, Status> {
        let ClosestPointRequest { coordinate } = request.into_inner();
        let point = match coordinate {
            Some(coordinate) => point! { x: coordinate.longitude, y: coordinate.latitude },
            None => return Err(Status::invalid_argument("Missing Coordinate")),
        };

        let nearest_point = self.graph.nearest_node(point).map_or(
            Err(Status::internal("Could not find appropriate point")),
            |coord| {
                Ok(Coordinate {
                    longitude: coord.position.x(),
                    latitude: coord.position.y(),
                })
            },
        )?;

        Ok(Response::new(ClosestPointResponse {
            coordinate: Some(nearest_point),
        }))
    }

    #[cfg_attr(feature="tracing", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn closest_snapped_point(
        &self,
        request: Request<ClosestSnappedPointRequest>,
    ) -> Result<Response<ClosestSnappedPointResponse>, Status> {
        let (_, _, request) = request.into_parts();

        let point = request
            .point
            .map(|v| Point(coord! { x: v.longitude, y: v.latitude }))
            .ok_or(Status::invalid_argument("Missing Point"))?;

        info!(
            "Got request for {} for nodes within {} square meters",
            point.wkt_string(),
            request.search_radius
        );

        let all_valids = self
            .graph
            .square_scan(&point, request.search_radius)
            .iter()
            .map(|p| p.position.wkt_string())
            .collect::<Vec<_>>()
            .join(", ");

        println!("All Valid Nodes: GEOMETRYCOLLECTION ({})", all_valids);

        let mut nearest_points = self
            .graph
            .nearest_projected_nodes(&point, request.search_radius)
            .collect::<Vec<_>>();

        debug!("Found {} points", nearest_points.len());

        println!(
            "Nearest points: GEOMETRYCOLLECTION ({})",
            nearest_points
                .iter()
                .map(|(p, ..)| p.wkt_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Get the closest of the discovered points
        nearest_points.sort_by(|(a, _), (b, _)| {
            let dist_to_a = point.haversine_distance(&a);
            let dist_to_b = point.haversine_distance(&b);
            dist_to_a.partial_cmp(&dist_to_b).unwrap_or(Ordering::Equal)
        });

        let nearest_point = nearest_points.first().map_or(
            Err(Status::internal("Could not find appropriate point")),
            |(coord, _)| {
                Ok(Coordinate {
                    longitude: coord.0.x,
                    latitude: coord.0.y,
                })
            },
        )?;

        Ok(Response::new(ClosestSnappedPointResponse {
            coordinate: Some(nearest_point),
        }))
    }
}
