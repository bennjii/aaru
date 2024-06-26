use std::collections::HashMap;
use axum::async_trait;
use bigtable_rs::bigtable::{BigTableConnection, RowCell};
use bigtable_rs::google::bigtable::v2::{ReadRowsRequest, RowFilter, RowRange, SampleRowKeysRequest};

use crate::tile::datasource::query::Query;
use crate::tile::error::TileError;
use crate::tile::querier::repositories::big_table;

pub const DEFAULT_APP_PROFILE: &'static str = "default";

pub type Repo = Box<dyn Repository>;

pub struct QuerySet {
    pub repositories: HashMap<String, Repo>
}

impl QuerySet {
    pub fn new() -> Self {
        Self {
            repositories: HashMap::new()
        }
    }

    pub fn get_repository(&self, repository: &str) -> Option<&Repo> {
        self.repositories.get(repository)
    }

    pub fn attach<R: Repository + 'static>(mut self, repository: R, name: &str) -> Self {
        self.repositories.insert(name.to_string(), Box::new(repository));
    }
}

#[async_trait]
pub trait Repository: Send + Sync {
    async fn new(project_id: &str, instance_name: &str, table_id: &str) -> Result<Self, TileError> where Self: Sized;
    async fn ping(&self) -> Result<(), TileError>;
    async fn query(&self, req: Query<Vec<RowRange>, Option<RowFilter>>) -> Result<Vec<(Vec<u8>, Vec<RowCell>)>, TileError>;
}

pub mod repositories {
    pub mod big_table {
        use std::time::Duration;
        use bigtable_rs::bigtable::BigTableConnection;

        pub(crate) const READ_ONLY: bool = true;
        pub(crate) const CHANNEL_SIZE: usize = 4;
        pub(crate) const TIMEOUT: Option<Duration> = Some(Duration::from_secs(20));

        pub struct BigTableRepository {
            pub connection: BigTableConnection,
            pub table_name: String
        }
    }
}

#[async_trait]
impl Repository for big_table::BigTableRepository {
    async fn new(project_id: &str, instance_name: &str, table_id: &str) -> Result<Self, TileError> where Self: Sized {
        let connection = BigTableConnection::new(
            project_id, instance_name, big_table::READ_ONLY, big_table::CHANNEL_SIZE, big_table::TIMEOUT
        ).await?;

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
        };

        client.sample_row_keys(req).await
            .map(|_| ())
            .map_err(|e| TileError::from(e))
    }

    async fn query(&self, req: Query<Vec<RowRange>, Option<RowFilter>>) -> Result<Vec<(Vec<u8>, Vec<RowCell>)>, TileError> {
        let mut client = self.connection.client();
        let request = ReadRowsRequest::from(req.add_param(self.table_name.clone()));
        client.read_rows(request).await.map_err(TileError::BigTableError)
    }
}