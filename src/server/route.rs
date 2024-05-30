use std::path::Path;
use tonic::{Request, Response, Status};

use router_service::{RouteRequest, RouteResponse};
use router_service::router_server::Router;
use crate::coord::latlng::{Degree, LatLng};
use crate::element::item::ProcessedElement::Node;
use crate::Graph;
use crate::server::route::router_service::Coordinate;

pub mod router_service {
    tonic::include_proto!("aaru");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("aaru_descriptor");
}

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
    async fn route(&self, request: Request<RouteRequest>) -> Result<Response<RouteResponse>, Status> {
        let (_, _, routing) = request.into_parts();

        let start = routing.start
            .map_or(
                Err(Status::invalid_argument("Missing Start")),
                |coord| Ok(LatLng::from(coord))
            )?;

        let end = routing.end
            .map_or(
                Err(Status::invalid_argument("Missing End")),
                |coord| Ok(LatLng::from(coord))
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

                    Ok(Response::new(RouteResponse { cost, shape }))
                }
            )
    }
}