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

    pub fn params(self) -> T {
        self.parameters
    }

    pub fn filter(self) -> F {
        self.filter
    }
}

pub trait Queryable<In, Filter, Out> {
    type Item;
    type Error;
    type Parameters;
    type Connection<'a> where Self: 'a;

    const QUERY_TABLE: &'static str;

    async fn query(&self, input: Query<In, Filter>, params: Self::Parameters) -> Result<Out, Self::Error>;
    fn batch(query: Query<Self::Parameters, ()>) -> In;
    fn filter(&self, filter: &Self::Parameters, item: &Self::Item) -> bool;
    fn connection(&self) -> Result<Self::Connection<'_>, Self::Error>;
}

impl From<Query<Vec<RowRange>, RowFilter>> for ReadRowsRequest {
    fn from(value: Query<Vec<RowRange>, RowFilter>) -> ReadRowsRequest {
        ReadRowsRequest {
            // TODO: Assign a table name
            table_name: "".to_string(),
            app_profile_id: DEFAULT_APP_PROFILE.to_string(),
            request_stats_view: 0, // new field, not sure what to do
            rows_limit: 0,         // boundless rows, implement limit via row filter / row-ranges

            filter: Some(value.filter),
            rows: Some(RowSet {
                row_keys: vec![],
                row_ranges: value.parameters,
            }),
            reversed: false,
        }
    }
}