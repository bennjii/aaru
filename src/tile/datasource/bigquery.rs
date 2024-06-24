use std::env;
use crate::tile::error::TileError;
use crate::tile::querier::repositories::big_table::BigTableRepository;
use crate::tile::querier::Repository;

fn get_env(key: &str) -> Result<String, TileError> {
    env::var(key).map_err(|e| TileError::MissingEnvironment(e.to_string()))
}

pub async fn init_bq() -> Result<BigTableRepository, TileError> {
    let project_id = get_env("BIGTABLE_PROJECT")?;
    let instance_name = get_env("BIGTABLE_INSTANCE")?;
    let table_id = get_env("BIGTABLE_TABLE")?;

    BigTableRepository::new(&project_id, &instance_name, &table_id).await
}