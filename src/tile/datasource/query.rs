use bigtable_rs::google::bigtable::v2::{ReadRowsRequest, RowFilter, RowRange, RowSet};
use crate::tile::querier::DEFAULT_APP_PROFILE;

pub struct Query<T, F> {
    parameters: T,
    filter: F
}

impl<T, F> Query<T, F> {
    pub fn new(parameters: T, filter: F) -> Self {
        Query { parameters, filter }
    }

    pub fn add_param<K>(self, new_param: K) -> Query<(T, K), F> {
        Query { parameters: (self.parameters, new_param), filter: self.filter }
    }

    pub fn params(&self) -> &T {
        &self.parameters
    }

    pub fn filter(&self) -> &F {
        &self.filter
    }
}

pub trait Queryable<In, Filter, Out> {
    type Item;
    type Error;
    type Parameters;
    type Connection<'a> where Self: 'a;

    const QUERY_TABLE: &'static str;

    async fn query(&self, input: Query<In, Option<Filter>>, params: Self::Parameters) -> Result<Out, Self::Error>;
    fn batch(&self, query: Query<Self::Parameters, (u8, u32, u32)>) -> In;
    fn filter(&self, filter: &Self::Parameters, item: &Self::Item) -> bool;
    fn connection(&self) -> Result<Self::Connection<'_>, Self::Error>;
}

impl From<Query<(Vec<RowRange>, String), Option<RowFilter>>> for ReadRowsRequest {
    fn from(value: Query<(Vec<RowRange>, String), Option<RowFilter>>) -> ReadRowsRequest {
        let (row_ranges, table_name) = value.parameters;

        ReadRowsRequest {
            table_name,
            app_profile_id: DEFAULT_APP_PROFILE.to_string(),
            request_stats_view: 0, // new field, not sure what to do
            rows_limit: 0,         // boundless rows, implement limit via row filter / row-ranges

            filter: value.filter,
            rows: Some(RowSet {
                row_keys: vec![],
                row_ranges,
            }),
            reversed: false,
        }
    }
}