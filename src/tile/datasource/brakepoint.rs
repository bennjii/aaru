use std::sync::Arc;

use axum::async_trait;
use axum::extract::{FromRequest, Path, State};
use axum::http::StatusCode;
use axum_macros::debug_handler;

use chrono::{DateTime, NaiveDateTime, Utc};
use scc::hash_map::OccupiedEntry;
use tracing::{debug, error, event, info, Level};
use prost::Message;
use serde::Deserialize;

use bigtable_rs::bigtable::RowCell;
use bigtable_rs::google::bigtable::v2::row_range::{EndKey, StartKey};
use bigtable_rs::google::bigtable::v2::{RowFilter, RowRange};
use tokio::time::Instant;
use tracing::field::debug;

use crate::geo::coord::latlng::LatLng;
use crate::geo::coord::point::Point;
use crate::codec::cvt::Brakepoint;
use crate::codec::mvt::{Layer, Tile, Value};
use crate::tile::datasource::date::format_date;
use crate::tile::datasource::query::{Query, Queryable};
use crate::tile::error::TileError;
use crate::tile::fragment::Fragment;
use crate::tile::params::QueryParams;
use crate::tile::querier::{QuerySet, Repo, Repository};
use crate::tile::querier::repositories::big_table::BigTableRepository;

const PREFIX: &str = "bp";
const STORAGE_ZOOM: u8 = 19;
const MIN_ZOOM: u8 = 16;

impl Point<Value, 2> for Brakepoint {
    fn id(&self) -> u64 {
        self.speed as u64
    }

    fn lat_lng(&self) -> LatLng {
        LatLng::from_degree_unchecked(self.latitude, self.longitude)
    }

    fn keys() -> [String; 2] {
        ["speed".to_string(), "gforce".to_string()]
    }

    fn values(&self) -> [Value; 2] {
        [Value::from_float(self.speed), Value::from_float(self.gforce)]
    }
}

impl Brakepoint {
    fn from_cell(cell: RowCell) -> Result<Self, TileError> {
        // TODO: Check this.
        // let qual = cell.qualifier.as_slice();
        // let seconds = i64::from_be_bytes(qual.try_into().unwrap_or_default());

        Brakepoint::decode(cell.value.as_slice()).map_err(TileError::ProtoDecode)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
enum RangeType {
    Inclusive,
    Exclusive
}

impl Default for RangeType {
    fn default() -> Self {
        RangeType::Inclusive
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
struct Range<T> {
    start: T,
    end: T,
    variant: RangeType
}

impl<T> Range<T> where T: PartialOrd {
    fn split(&self) -> (&T, &T) {
        (&self.start, &self.end)
    }

    fn within(&self, other: T) -> bool {
        match self.variant {
            RangeType::Exclusive => self.start < other && self.end > other,
            RangeType::Inclusive => self.start <= other && self.end >= other
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct BrakepointParams {
    date: Range<DateTime<Utc>>,
    gforce: Range<f32>,
    speed: Range<f32>,
}

impl Queryable<Vec<RowRange>, RowFilter, Tile> for QuerySet {
    type Item = Brakepoint;
    type Error = TileError;
    type Parameters = (BrakepointParams, u8);
    type Connection<'a> = &'a Repo;

    const QUERY_TABLE: &'static str = "big_table";

    async fn query(&self, input: Query<Vec<RowRange>, Option<RowFilter>>, parameters: Self::Parameters) -> Result<Tile, Self::Error> {
        let connection = self.connection()?;

        tracing::event!(Level::INFO, name="query::start", table=Self::QUERY_TABLE);
        let rows = connection.query(input).await?;
        tracing::event!(Level::INFO, name="query::end", table=Self::QUERY_TABLE);

        if rows.len() == 0 {
            info!("Found no tile data.");
            return Err(TileError::NoTilesFound)
        }

        info!("Got tile data, processing...");

        let points: Vec<Brakepoint> = rows
            .into_iter()
            .flat_map(|r| r.1)
            .map(Brakepoint::from_cell)
            .filter_map(|r| r.ok())
            .filter(|point| self.filter(&parameters, point))
            .collect();

        Ok(Tile::from(Layer::from((points, parameters.1))))
    }

    fn batch(&self, query: Query<Self::Parameters, (u8, u32, u32)>) -> Vec<RowRange> {
        let (start_date, end_date) = query.params().0.date.split();
        let (z, x, y) = query.filter();

        let format_key = |date: &DateTime<Utc>, hid: u64| {
            format!("{:012}/{}/{}", hid, PREFIX, format_date(date)).into_bytes()
        };

        debug!("Input fragment: {:?}", Fragment::new(z, x, y));

        Fragment::new(z, x, y)
            .detail(STORAGE_ZOOM)
            .into_iter()
            .inspect(|fragment| {
                debug!("Obtaining fragment data: {:?}", fragment)
            })
            .map(|t| RowRange {
                start_key: Some(StartKey::StartKeyClosed(format_key(
                    &start_date,
                    t.to_hilbert(),
                ))),
                end_key: Some(EndKey::EndKeyClosed(format_key(
                    &end_date,
                    t.to_hilbert(),
                ))),
            })
            .collect()
    }

    fn filter(&self, (filter, _): &Self::Parameters, item: &Self::Item) -> bool {
        filter.gforce.within(item.gforce)
            && filter.speed.within(item.speed)
    }

    fn connection(&self) -> Result<Self::Connection<'_>, Self::Error> {
        self.get_repository(Self::QUERY_TABLE)
            .ok_or(TileError::NoMatchingRepository)
    }
}

impl Brakepoint {
    pub async fn query(
        State(state): State<Arc<QuerySet>>,
        Path((z, x, y)): Path<(u8, u32, u32)>,
        QueryParams(params): QueryParams<BrakepointParams>
    ) -> Result<Tile, TileError> {
        event!(Level::TRACE, "query::spawn", loc=(z,x,y), params=params);

        if z < MIN_ZOOM || z > STORAGE_ZOOM {
            event!(Level::ERROR, "zoom::unsupported", zoom=%z);
            return Err(TileError::UnsupportedZoom(z));
        }

        let rows = state.batch(
            Query::new((params, z), (z, x, y))
        );

        event!(Level::INFO, name="query::plan", size=rows.len());
        state.query(Query::new(rows, None), (params, z)).await
    }
}