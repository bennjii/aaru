use axum::async_trait;
use std::env;

use bigtable_rs::bigtable::{BigTableConnection, RowCell};
use bigtable_rs::google::bigtable::v2::{
    ReadRowsRequest, RowFilter, RowRange, RowSet, SampleRowKeysRequest,
};

use crate::error::TileError;
use crate::repository::{DEFAULT_APP_PROFILE, Repository};
use crate::{Query, RepositorySet};

use super::repositories::big_table;
use super::repositories::big_table::BigTableRepository;

type RowKey = Vec<u8>;
pub type BigTableOutput = Vec<(RowKey, Vec<RowCell>)>;
pub type BigTableInput = Query<Vec<RowRange>, Option<RowFilter>>;
pub type BigTableRepositorySet = RepositorySet<BigTableInput, BigTableOutput>;

#[async_trait]
impl Repository<BigTableInput, BigTableOutput> for BigTableRepository {
    async fn new(project_id: &str, instance_name: &str, table_id: &str) -> Result<Self, TileError>
    where
        Self: Sized,
    {
        let connection = BigTableConnection::new(
            project_id,
            instance_name,
            big_table::READ_ONLY,
            big_table::CHANNEL_SIZE,
            big_table::TIMEOUT,
        )
        .await
        .map_err(|e| TileError::DataSourceError(e.to_string()))?;

        let client = connection.client();

        Ok(Self {
            connection,
            table_name: client.get_full_table_name(table_id),
        })
    }

    async fn ping(&self) -> Result<(), TileError> {
        let mut client = self.connection.client();

        let req = SampleRowKeysRequest {
            table_name: self.table_name.clone(),
            app_profile_id: DEFAULT_APP_PROFILE.to_string(),
            ..SampleRowKeysRequest::default()
        };

        client
            .sample_row_keys(req)
            .await
            .map(|_| ())
            .map_err(|e| TileError::DataSourceError(e.to_string()))
    }

    async fn query(&self, req: BigTableInput) -> Result<BigTableOutput, TileError> {
        let mut client = self.connection.client();

        let request = ReadRowsRequest::from(BigTableQuery(req.add_param(self.table_name.clone())));

        client
            .read_rows(request)
            .await
            .map_err(|err| TileError::DataSourceError(err.to_string()))
    }
}

pub struct BigTableQuery(pub Query<(Vec<RowRange>, String), Option<RowFilter>>);

impl From<BigTableQuery> for ReadRowsRequest {
    fn from(value: BigTableQuery) -> ReadRowsRequest {
        let (row_ranges, table_name) = value.0.parameters;

        ReadRowsRequest {
            table_name,

            app_profile_id: DEFAULT_APP_PROFILE.to_string(),
            request_stats_view: 0, // new field, not sure what to do
            rows_limit: 0,         // boundless rows, implement limit via row filter / row-ranges

            filter: value.0.filter,
            rows: Some(RowSet {
                row_keys: vec![],
                row_ranges,
            }),
            reversed: false,

            ..ReadRowsRequest::default()
        }
    }
}

fn get_env(key: &str) -> Result<String, TileError> {
    env::var(key).map_err(|e| TileError::MissingEnvironment(e.to_string()))
}

pub async fn init_bq() -> Result<BigTableRepository, TileError> {
    let project_id = get_env("BIGTABLE_PROJECT")?;
    let instance_name = get_env("BIGTABLE_INSTANCE")?;
    let table_id = get_env("BIGTABLE_TABLE")?;

    BigTableRepository::new(&project_id, &instance_name, &table_id).await
}
