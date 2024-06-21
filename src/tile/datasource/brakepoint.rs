use bigtable_rs::bigtable::RowCell;
use bigtable_rs::google::bigtable::v2::row_range::{EndKey, StartKey};
use bigtable_rs::google::bigtable::v2::{RowFilter, RowRange};
use chrono::{DateTime, Utc};
use prost::Message;
use scc::hash_map::OccupiedEntry;

use crate::geo::coord::latlng::LatLng;
use crate::geo::coord::point::Point;
use crate::codec::cvt::Brakepoint;
use crate::codec::mvt::{Layer, Tile, Value};
use crate::tile::datasource::date::format_date;
use crate::tile::datasource::query::{Query, Queryable};
use crate::tile::error::TileError;
use crate::tile::fragment::Fragment;
use crate::tile::querier::{QuerySet, Repository};

const PREFIX: &str = "bp";
const STORAGE_ZOOM: u8 = 19;
const MIN_ZOOM: u8 = 16;

impl Point<Value, 2> for Brakepoint {
    const ZOOM: u8 = 0;

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
        let qual = cell.qualifier.as_slice();
        let seconds = i64::from_be_bytes(qual.try_into().unwrap_or_default());

        Brakepoint::decode(cell.value.as_slice()).map_err(TileError::ProtoDecode)
    }
}

pub struct BrakepointParams {
    date: (DateTime<Utc>, DateTime<Utc>),
    gforce: (f32, f32),
    speed: (f32, f32),
    position: (u8, u32, u32),
}

impl Queryable<Vec<RowRange>, RowFilter, Tile> for QuerySet {
    type Item = Brakepoint;
    type Error = TileError;
    type Parameters = BrakepointParams;
    type Connection<'a> = OccupiedEntry<'a, String, Repository>;

    // TODO: Implement Me!
    const QUERY_TABLE: &'static str = "";

    async fn query(&self, input: Query<Vec<RowRange>, RowFilter>, parameters: Self::Parameters) -> Result<Tile, Self::Error> {
        let connection = self.connection()?;
        let rows = connection.query(input).await?;

        if rows.len() == 0 {
            return Err(TileError::NoTilesFound)
        }

        let points: Vec<Brakepoint> = rows
            .into_iter()
            .flat_map(|r| r.1)
            .map(Brakepoint::from_cell)
            .filter_map(|r| r.ok())
            .filter(|point| self.filter(&parameters, point))
            .collect();

        Ok(Tile::from(Layer::from(points)))
    }

    fn batch(query: Query<BrakepointParams, ()>) -> Vec<RowRange> {
        let params = query.params();
        let (start_date, end_date) = params.date;
        let (z, x, y) = params.position;

        let format_key = |date: &DateTime<Utc>, hid: u64| {
            format!("{:012}/{}/{}", hid, PREFIX, format_date(date)).into_bytes()
        };

        Fragment::new(z, x, y)
            .detail(STORAGE_ZOOM)
            .into_iter()
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

    fn filter(&self, filter: &Self::Parameters, item: &Self::Item) -> bool {
        item.gforce >= filter.gforce.0
            && item.gforce <= filter.gforce.1
            && item.speed >= filter.speed.0
            && item.speed <= filter.speed.1
    }

    fn connection(&self) -> Result<Self::Connection<'_>, Self::Error> {
        self.get_repository(Self::QUERY_TABLE)
            .ok_or(TileError::NoMatchingRepository)
    }
}