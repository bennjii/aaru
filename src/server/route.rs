use std::path::Path;
use log::debug;
use rstar::PointDistance;
use tonic::{Request, Response, Status};

use router_service::{RouteRequest, RouteResponse};
use router_service::router_server::Router;

#[cfg(feature = "tracing")]
use tracing::Level;

use crate::server::route::router_service::{ClosestSnappedPointRequest, Coordinate, MapMatchRequest, MapMatchResponse};
use crate::geo::coord::latlng::{LatLng};
use crate::route::Graph;

pub mod router_service {
    tonic::include_proto!("aaru");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("aaru_descriptor");
}

#[derive(Debug)]
pub struct RouteService {
    graph: Graph
}

impl RouteService {
    pub fn from_file(file: &str) -> crate::Result<RouteService> {
        let path = Path::new(file);
        let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

        Ok(RouteService { graph })
    }
}

#[tonic::async_trait]
impl Router for RouteService {
    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::INFO)))]
    async fn map_match(&self, request: Request<MapMatchRequest>) -> Result<Response<MapMatchResponse>, Status> {
        let mapmatch = request.into_inner();
        let coordinates = mapmatch.data.iter()
            .map(|coord| LatLng::try_from(Some(*coord)))
            .collect::<Result<Vec<_>, Status>>()?;

        let matched = self.graph.map_match(coordinates, mapmatch.distance as i64);

        Ok(Response::new(MapMatchResponse {
            matched: matched.iter()
                .map(|node| Coordinate {
                    latitude: node.lat(),
                    longitude: node.lng()
                })
                .collect::<Vec<_>>()
        }))
    }

    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::INFO)))]
    async fn route(&self, request: Request<RouteRequest>) -> Result<Response<RouteResponse>, Status> {
        let (_, _, routing) = request.into_parts();

        let start = LatLng::try_from(routing.start)?;
        let end = LatLng::try_from(routing.end)?;

        self.graph.route(start, end)
            .map_or(
                Err(Status::internal("Could not route")),
                |(cost, route)| {
                    let shape = route
                        .iter()
                        .map(|node| Coordinate {
                            latitude: node.position.lat(),
                            longitude: node.position.lng()
                        })
                        .collect();

                    Ok(Response::new(RouteResponse { cost: cost as u32, shape }))
                }
            )
    }

    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::INFO)))]
    async fn closest_point(&self, request: Request<Coordinate>) -> Result<Response<Coordinate>, Status> {
        let point = LatLng::try_from(Some(request.into_inner()))?;

        let nearest_point = self.graph.nearest_node(point)
            .map_or(
                Err(Status::internal("Could not find appropriate point")),
                |coord| Ok(coord.position.coordinate())
            )?;

        Ok(Response::new(nearest_point))
    }

    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::INFO)))]
    async fn closest_snapped_point(&self, request: Request<ClosestSnappedPointRequest>) -> Result<Response<Coordinate>, Status> {
        let (_, _, request) = request.into_parts();

        let point = LatLng::try_from(request.point)?;
        let distance_as_degree: i64 = 1e7 as i64 * (request.distance as i64);
        let mut nearest_points = self.graph.nearest_projected_nodes(&point, distance_as_degree)
            .collect::<Vec<_>>();

        debug!("Found {} points", nearest_points.len());

        // Get the closest of the discovered points
        nearest_points.sort_by(|(a, _), (b, _)| {
            point.distance_2(a).cmp(&point.distance_2(b))
        });

        let nearest_point = nearest_points.get(0)
            .map_or(
                Err(Status::internal("Could not find appropriate point")),
                |(coord, _)| Ok(coord.coordinate())
            )?;

        Ok(Response::new(nearest_point))
    }
}