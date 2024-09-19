use std::path::Path;
use log::debug;
use rstar::PointDistance;
use tonic::{Request, Response, Status};

use router_service::{RouteRequest, RouteResponse};
use router_service::router_server::Router;

#[cfg(feature = "tracing")]
use tracing::Level;

use crate::geo::coord::latlng::{LatLng};
use crate::route::Graph;
use crate::server::route::router_service::{ClosestSnappedPointRequest, Coordinate};

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
    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::ERROR)))]
    async fn route(&self, request: Request<RouteRequest>) -> Result<Response<RouteResponse>, Status> {
        let (_, _, routing) = request.into_parts();

        let start = routing.start
            .map_or(
                Err(Status::invalid_argument("Missing Start")),
                |coord| LatLng::try_from(coord)
                    .map_err(|err| Status::internal(format!("{:?}", err)))
            )?;

        let end = routing.end
            .map_or(
                Err(Status::invalid_argument("Missing End")),
                |coord| LatLng::try_from(coord)
                    .map_err(|err| Status::internal(format!("{:?}", err)))
            )?;

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

    #[cfg_attr(feature="tracing", tracing::instrument(err(level = Level::ERROR)))]
    async fn closest_point(&self, request: Request<Coordinate>) -> Result<Response<Coordinate>, Status> {
        let point = LatLng::try_from(request.into_inner())
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        let nearest_point = self.graph.nearest_node(point)
            .map_or(
                Err(Status::internal("Could not find appropriate point")),
                |coord| Ok(coord.position.coordinate())
            )?;

        Ok(Response::new(nearest_point))
    }

    async fn closest_snapped_point(&self, request: Request<ClosestSnappedPointRequest>) -> Result<Response<Coordinate>, Status> {
        let (_, _, request) = request.into_parts();

        let point = request.point
            .map_or(
                Err(Status::invalid_argument("Missing Point")),
                |coord| LatLng::try_from(coord)
                    .map_err(|err| Status::internal(format!("{:?}", err)))
            )?;

        let distance_as_degree: i64 = 1e7 as i64 * (request.distance as i64);
        let mut nearest_points = self.graph.nearest_projected_nodes(point, distance_as_degree)
            .collect::<Vec<_>>();

        // Get the closest of the discovered points
        nearest_points.sort_by(|a, b| {
            debug!("DistA={}. DistB={}", point.distance_2(a) as f64 / 10e7, point.distance_2(b) as f64 / 10e7);
            point.distance_2(a).cmp(&point.distance_2(b))
        });

        let nearest_point = nearest_points.get(0)
            .map_or(
                Err(Status::internal("Could not find appropriate point")),
                |coord| Ok(coord.coordinate())
            )?;

        Ok(Response::new(nearest_point))
    }
}