use crate::definition::model::*;
use crate::definition::proximity::*;
use crate::services::RouteService;

use routers::Proximity;

use geo::{Distance, Haversine, Point, coord, point};
use log::{debug, info};
use std::cmp::Ordering;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use wkt::ToWkt;

#[cfg(feature = "telemetry")]
use tracing::Level;

#[tonic::async_trait]
impl ProximityService for Arc<RouteService> {
    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn proximal_point(
        self: Arc<Self>,
        request: Request<ProximalRequest>,
    ) -> Result<Response<ProximalPointResponse>, Status> {
        let ProximalRequest { coordinate } = request.into_inner();
        let point = match coordinate {
            Some(coordinate) => point! { x: coordinate.longitude, y: coordinate.latitude },
            None => return Err(Status::invalid_argument("Missing Coordinate")),
        };

        let nearest_point = self.graph.proximal_node(point).map_or(
            Err(Status::internal("Could not find appropriate point")),
            |coord| {
                Ok(Coordinate {
                    longitude: coord.position.x(),
                    latitude: coord.position.y(),
                })
            },
        )?;

        Ok(Response::new(ProximalPointResponse {
            coordinate: Some(nearest_point),
        }))
    }

    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn proximal_edge(
        self: Arc<Self>,
        _request: Request<ProximalRequest>,
    ) -> Result<Response<ProximalEdgeResponse>, Status> {
        unimplemented!()
    }

    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, err(level = Level::INFO)))]
    async fn proximal_point_snapped(
        self: Arc<Self>,
        request: Request<ProximalSnappedRequest>,
    ) -> Result<Response<ProximalPointSnappedResponse>, Status> {
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
            .proximal_nodes(&point, request.search_radius)
            .map(|p| p.position.wkt_string())
            .collect::<Vec<_>>()
            .join(", ");

        println!("All Valid Nodes: GEOMETRYCOLLECTION ({})", all_valids);

        let mut nearest_points = self
            .graph
            .proximal_nodes_projected(&point, request.search_radius)
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
            let dist_to_a = Haversine.distance(point, *a);
            let dist_to_b = Haversine.distance(point, *b);
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

        Ok(Response::new(ProximalPointSnappedResponse {
            coordinate: Some(nearest_point),
        }))
    }
}
