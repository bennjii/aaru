use geo::{Coord, Distance, Geodesic, Point};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::definition::r#match::*;
use crate::definition::model::*;

use crate::services::RouteService;
use codec::{Entry, Metadata};
use routers::{Match, Path, RoutedPath};
#[cfg(feature = "telemetry")]
use tracing::Level;

struct Util;

impl Util {
    fn coordinate_from_point(point: Point) -> Coordinate {
        <geo::Point as Into<Coord>>::into(point).into()
    }

    fn route_from_path<E: Entry, M: Metadata>(input: Path<E, M>) -> Vec<RouteElement>
    where
        EdgeMetadata: for<'a> From<&'a M>,
    {
        input
            .iter()
            .flat_map(|entry| {
                let edge = EdgeBuilder::default()
                    .id(entry.edge.id().identifier())
                    .source(entry.edge.source)
                    .target(entry.edge.target)
                    .metadata(EdgeMetadata::from(&entry.metadata))
                    .length(
                        Geodesic.distance(entry.edge.source.position, entry.edge.target.position),
                    )
                    .build()
                    .unwrap();

                RouteElementBuilder::default()
                    .coordinate(Util::coordinate_from_point(entry.point))
                    .edge(RouteEdge {
                        edge: Some(edge),
                        ..RouteEdge::default()
                    })
                    .build()
            })
            .collect::<Vec<_>>()
    }

    fn process<E: Entry, M: Metadata>(result: RoutedPath<E, M>) -> Vec<MatchedRoute>
    where
        EdgeMetadata: for<'a> From<&'a M>,
    {
        let interpolated = Util::route_from_path(result.interpolated);
        let discretized = Util::route_from_path(result.discretized);

        let matched_route = MatchedRoute {
            interpolated,
            discretized,
            cost: 0,
        };

        vec![matched_route]
    }
}

#[tonic::async_trait]
impl<E, M> MatchService for RouteService<E, M>
where
    M: Metadata + 'static,
    E: Entry + 'static,
    EdgeMetadata: for<'a> From<&'a M>,
{
    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn r#match(
        self: Arc<Self>,
        request: Request<MatchRequest>,
    ) -> Result<Response<MatchResponse>, Status> {
        let map_match = request.into_inner();
        let coordinates = map_match.linestring();

        let result = self
            .graph
            .r#match(coordinates)
            .map_err(|e| e.to_string())
            .map_err(Status::internal)?;

        // TODO: Vector to allow trip-splitting in the future.
        Ok(Response::new(MatchResponse {
            matches: Util::process(result),
        }))
    }

    #[cfg_attr(feature="telemetry", tracing::instrument(skip_all, level = Level::INFO))]
    async fn snap(
        self: Arc<Self>,
        request: Request<SnapRequest>,
    ) -> Result<Response<SnapResponse>, Status> {
        let map_match = request.into_inner();
        let coordinates = map_match.linestring();

        let result = self
            .graph
            .snap(coordinates)
            .map_err(|e| e.to_string())
            .map_err(Status::internal)?;

        Ok(Response::new(SnapResponse {
            matches: Util::process(result),
        }))
    }
}
