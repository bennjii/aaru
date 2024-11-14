use geo::{coord, point, Point};
use log::{debug, info};
use rstar::PointDistance;
use std::cmp::Ordering;
use std::path::Path;
use tonic::{Request, Response, Status};

use router_service::{RouteRequest, RouteResponse};

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
            .map(|coord| coord! { x: coord.longitude, y: coord.longitude })
            .collect::<LineString>();

        let linestring = self.graph.map_match(coordinates, mapmatch.distance);

        Ok(Response::new(MapMatchResponse {
            matched: linestring
                .coords()
                .map(|node| Coordinate {
                    latitude: node.y,
                    longitude: node.x,
                })
                .collect::<Vec<_>>(),
            linestring: linestring.wkt_string(),
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
            .map_or(Err(Status::invalid_argument("Missing Point")), |v| {
                Ok(Point(coord! { x: v.longitude, y: v.latitude }))
            })
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        info!(
            "Got request for {} for distances <= {}",
            point.wkt_string(),
            request.quantity
        );
        let mut nearest_points = self
            .graph
            .nearest_projected_nodes(&point, request.quantity as usize)
            .collect::<Vec<_>>();

        debug!("Found {} points", nearest_points.len());

        // Get the closest of the discovered points
        nearest_points.sort_by(|(a, _), (b, _)| {
            let dist_to_a = point.0.distance_2(&a.0);
            let dist_to_b = point.0.distance_2(&b.0);
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
