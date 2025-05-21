use axum::async_trait;
use axum::extract::FromRequest;
use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde_qs::Config;

// Extractor for `serde_qs` support
pub struct QueryParams<T>(pub T);

#[async_trait]
impl<B, T> FromRequest<B> for QueryParams<T>
where
    B: Send,
    T: DeserializeOwned + Clone + Send + 'static,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: axum::extract::Request, _: &B) -> Result<Self, Self::Rejection> {
        let uri = req.uri();
        let query = uri
            .query()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing query string".to_string()))?;

        let config = Config::new(5, false);
        let params: T = config.deserialize_str(query).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to deserialize query string: {}", err),
            )
        })?;

        Ok(QueryParams(params))
    }
}
