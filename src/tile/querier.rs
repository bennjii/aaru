use std::time::Duration;
use bigtable_rs::bigtable::{BigTableConnection, RowCell};
use bigtable_rs::google::bigtable::v2::{ReadRowsRequest, RowFilter, RowRange, SampleRowKeysRequest};
use scc::hash_map::OccupiedEntry;
use scc::HashMap;
use tonic::codegen::tokio_stream::StreamExt;
use crate::tile::datasource::query::Query;
use crate::tile::error::TileError;

pub const DEFAULT_APP_PROFILE: &'static str = "default";

pub struct QuerySet {
    pub repositories: HashMap<String, Repository>
}

impl QuerySet {
    pub fn new() -> Self {
        Self {
            repositories: HashMap::new()
        }
    }

    pub fn get_repository(&self, repository: &str) -> Option<OccupiedEntry<String, Repository>> {
        self.repositories.get(repository)
    }
}

pub struct Repository {
    connection: BigTableConnection,
    table_name: String
}

impl Repository {
    const READ_ONLY: bool = true;
    const CHANNEL_SIZE: usize = 4;
    const TIMEOUT: Option<Duration> = Some(Duration::from_secs(20));

    pub async fn new(project_id: &str, instance_name: &str, table_id: &str) -> Result<Self, TileError> {
        let connection = BigTableConnection::new(
            project_id, instance_name, Self::READ_ONLY, Self::CHANNEL_SIZE, Self::TIMEOUT
        ).await?;

        let client = connection.client();

        Ok(Self {
            connection,
            table_name: client.get_full_table_name(table_id),
        })
    }

    pub async fn ping(&self) -> Result<(), TileError> {
        let mut client = self.connection.client();

        let req = SampleRowKeysRequest {
            table_name: self.table_name.clone(),
            app_profile_id: DEFAULT_APP_PROFILE.to_string(),
        };

        client.sample_row_keys(req).await
            .map(|_| ())
            .map_err(|e| TileError::from(e))
    }

    pub async fn query(&self, req: Query<Vec<RowRange>, Option<RowFilter>>) -> Result<Vec<(Vec<u8>, Vec<RowCell>)>, TileError> {
        let mut client = self.connection.client();
        let request = ReadRowsRequest::from(req);

        client.read_rows(request).await.map_err(TileError::BigTableError)
    }
}