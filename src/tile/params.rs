use axum::async_trait;
use axum::extract::FromRequest;
use axum::http::StatusCode;
use serde_qs::Config;

use crate::tile::datasource::brakepoint::BrakepointParams;

// Extractor for `serde_qs` support
pub struct QueryParams<T>(pub T);

#[async_trait]
impl<B> FromRequest<B> for QueryParams<BrakepointParams>
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: axum::extract::Request, state: &B) -> Result<Self, Self::Rejection> {
        let uri = req.uri();
        let query = uri.query().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing query string".to_string()
            )
        })?;

        let config = Config::default();
        let params: BrakepointParams = config.deserialize_str(query).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to deserialize query string: {}", err)
            )
        })?;

        Ok(QueryParams(params))
    }
}