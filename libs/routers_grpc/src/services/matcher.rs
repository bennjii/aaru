use geo::{LineString, coord};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::definition::r#match::*;
use crate::definition::model::*;

use crate::services::RouteService;
use routers::Match;
#[cfg(feature = "telemetry")]
use tracing::Level;

#[tonic::async_trait]
impl MatchService for Arc<RouteService> {
    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn r#match(
        self: Arc<Self>,
        request: Request<MatchRequest>,
    ) -> Result<Response<MatchResponse>, Status> {
        let map_match = request.into_inner();
        let coordinates = map_match
            .data
            .iter()
            .map(|coord| coord! { x: coord.longitude, y: coord.latitude })
            .collect::<LineString>();

        let result = self
            .graph
            .map_match(coordinates)
            .map_err(|err| Status::internal(format!("{:?}", err)))?;

        let snapped_shape = result
            .matched()
            .iter()
            .map(|node| Coordinate {
                latitude: node.position.y(),
                longitude: node.position.x(),
            })
            .collect::<Vec<_>>();

        let interpolated = result
            .interpolated(&self.graph)
            .coords()
            .map(|node| Coordinate {
                latitude: node.y,
                longitude: node.x,
            })
            .collect::<Vec<_>>();

        // TODO: Correctly updraw this
        let matching = MatchedRoute {
            snapped_shape,
            interpolated,

            edges: vec![],
            label: "!".to_string(),
            cost: 0,
        };

        Ok(Response::new(MatchResponse {
            // TODO: Vector to allow trip-splitting in the future.
            matches: vec![matching],
            // TODO: Aggregate all the errored trips.
            warnings: vec![],
        }))
    }

    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn snap(
        self: Arc<Self>,
        _request: Request<SnapRequest>,
    ) -> Result<Response<SnapResponse>, Status> {
        unimplemented!()
    }
}
