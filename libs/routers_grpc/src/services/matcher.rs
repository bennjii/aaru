use std::ops::Deref;
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::definition::r#match::*;
use crate::definition::model::*;

use crate::services::RouteService;
use codec::Entry;
use routers::{Match, RoutedPath};
#[cfg(feature = "telemetry")]
use tracing::Level;

struct Util;

impl Util {
    fn process<E: Entry, Meta>(
        _service: impl Deref<Target = RouteService<E>>,
        _result: RoutedPath<E, Meta>,
    ) -> Vec<MatchedRoute> {
        unimplemented!();

        // let snapped_shape = result
        //     .matched()
        //     .iter()
        //     .map(|node| Coordinate {
        //         latitude: node.position.y(),
        //         longitude: node.position.x(),
        //     })
        //     .collect::<Vec<_>>();
        //
        // let interpolated = result
        //     .interpolated(&service.graph)
        //     .coords()
        //     .map(|node| Coordinate {
        //         latitude: node.y,
        //         longitude: node.x,
        //     })
        //     .collect::<Vec<_>>();
        //
        // // TODO: Correctly updraw this
        // let matching = MatchedRoute {
        //     snapped_shape,
        //     interpolated,
        //
        //     edges: vec![],
        //     label: "!".to_string(),
        //     cost: 0,
        // };
        //
        // vec![matching]
    }
}

#[tonic::async_trait]
// TODO: Arc:Arc - Remove double usage.
impl<E> MatchService for RouteService<E>
where
    E: Entry + 'static,
{
    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn r#match(
        self: Arc<Self>,
        request: Request<MatchRequest>,
    ) -> Result<Response<MatchResponse>, Status> {
        let map_match = request.into_inner();
        let coordinates = map_match.linestring();

        let result: RoutedPath<_, ()> = self
            .graph
            .r#match(coordinates)
            .map_err(|e| e.to_string())
            .map_err(Status::internal)?;

        Ok(Response::new(MatchResponse {
            // TODO: Vector to allow trip-splitting in the future.
            matches: Util::process(self.deref(), result),
        }))
    }

    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn snap(
        self: Arc<Self>,
        request: Request<SnapRequest>,
    ) -> Result<Response<SnapResponse>, Status> {
        let map_match = request.into_inner();
        let coordinates = map_match.linestring();

        let result: RoutedPath<_, ()> = self
            .graph
            .snap(coordinates)
            .map_err(|e| e.to_string())
            .map_err(Status::internal)?;

        Ok(Response::new(SnapResponse {
            matches: Util::process(self.deref(), result),
        }))
    }
}
