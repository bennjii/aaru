use crate::error::TileError;
use async_trait::async_trait;
use std::collections::HashMap;

pub const DEFAULT_APP_PROFILE: &str = "default";

pub type Repo<I, O> = Box<dyn Repository<I, O>>;

pub struct RepositorySet<I, O> {
    pub repositories: HashMap<String, Repo<I, O>>,
}

impl<I, O> Default for RepositorySet<I, O> {
    fn default() -> Self {
        Self {
            repositories: HashMap::new(),
        }
    }
}

impl<I, O> RepositorySet<I, O> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_repository(&self, repository: &str) -> Option<&Repo<I, O>> {
        self.repositories.get(repository)
    }

    pub fn attach<R: Repository<I, O> + 'static>(mut self, repository: R, name: &str) -> Self {
        self.repositories
            .insert(name.to_string(), Box::new(repository));
        self
    }
}

#[async_trait]
pub trait Repository<Input, Output>: Send + Sync {
    async fn new(project_id: &str, instance_name: &str, table_id: &str) -> Result<Self, TileError>
    where
        Self: Sized;
    async fn ping(&self) -> Result<(), TileError>;
    async fn query(&self, req: Input) -> Result<Output, TileError>;
}
