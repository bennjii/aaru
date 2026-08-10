//! Staples aggregation metadata onto every RPC response.
//!
//! Downstream platforms aggregate routers queries to understand usage:
//! which tool a query belongs to (its *grouping*) and whether two queries
//! are the same request repeated (its *hash*). The [`QueryMetadata`]
//! interceptor derives both for every query served and returns them as
//! response headers, so callers can forward them to their own telemetry
//! without inspecting request payloads themselves:
//!
//! - [`GROUPING_HEADER`] (`x-rpc-grouping`) carries a stable, human-readable
//!   label for the tool the RPC belongs to, e.g. `path-analysis`.
//! - [`QUERY_HASH_HEADER`] (`x-query-hash`) carries a hex-encoded SHA-256
//!   digest over the RPC path and the wire-encoded request payload, so
//!   repeated identical queries can be aggregated by their hash.
//!
//! Register the interceptor when constructing the server:
//!
//! ```rust,ignore
//! Server::new(router)
//!     .with_interceptor(QueryMetadata)
//!     .serve(addr)
//!     .await?;
//! ```

use alloc::borrow::Cow;
use core::fmt::Write;

use connectrpc::{
    ConnectError, Interceptor, Next, NextStream, PayloadStream, StreamRequest, StreamResponse,
    UnaryRequest, UnaryResponse,
};
use sha2::{Digest, Sha256};

/// Response header carrying the query's grouping label.
pub const GROUPING_HEADER: &str = "x-rpc-grouping";

/// Response header carrying the query's request hash.
pub const QUERY_HASH_HEADER: &str = "x-query-hash";

/// Grouping labels for the services this crate serves.
///
/// Services not listed here fall back to a label derived from the service
/// name (see [`grouping`]), so new services are labelled without a code
/// change — add an entry when a service warrants a curated label.
const GROUPINGS: &[(&str, &str)] = &[
    ("routers.api.match.v1.MatchService", "path-analysis"),
    (
        "routers.api.optimise.v1.OptimiseService",
        "route-optimisation",
    ),
    ("routers.api.scan.v1.ScanService", "proximity-scan"),
    ("routers.api.timezone.v1.TimezoneService", "timezone-lookup"),
];

/// The grouping label for an RPC path (`/package.Service/Method`).
///
/// Known services map through [`GROUPINGS`]; anything else derives a
/// kebab-case label from the service's unqualified name, dropping a
/// `Service` suffix — `/example.v1.PathAnalysisService/Run` becomes
/// `path-analysis`. Returns `None` when no label can be derived.
pub fn grouping(path: &str) -> Option<Cow<'static, str>> {
    let service = path.trim_start_matches('/').split('/').next()?;

    if let Some((_, group)) = GROUPINGS.iter().find(|(name, _)| *name == service) {
        return Some(Cow::Borrowed(group));
    }

    let name = service.rsplit('.').next()?;
    let name = name.strip_suffix("Service").unwrap_or(name);

    let mut label = String::with_capacity(name.len() + 4);
    for (position, char) in name.char_indices() {
        if char.is_ascii_uppercase() {
            if position != 0 {
                label.push('-');
            }

            label.push(char.to_ascii_lowercase());
        } else if char.is_ascii_alphanumeric() {
            label.push(char);
        }
    }

    (!label.is_empty()).then_some(Cow::Owned(label))
}

/// The hash identifying a query: a hex-encoded SHA-256 digest over the RPC
/// path and the wire-encoded request payload.
///
/// Hashing the path alongside the payload keeps identical payloads sent to
/// different methods distinct. The digest is over the payload's wire bytes,
/// so the same query is only recognised as repeated when sent with the same
/// codec (proto or JSON) — which holds for any given caller.
pub fn query_hash(path: &str, payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update([0u8]);
    hasher.update(payload);

    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to a String cannot fail");
            hex
        })
}

/// Interceptor stapling [`GROUPING_HEADER`] and [`QUERY_HASH_HEADER`] onto
/// every response.
///
/// Streaming RPCs receive only the grouping label — their payload is not
/// buffered, so no hash is computed.
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryMetadata;

#[connectrpc::async_trait]
impl Interceptor for QueryMetadata {
    async fn intercept_unary(
        &self,
        req: UnaryRequest,
        next: Next<'_>,
    ) -> Result<UnaryResponse, ConnectError> {
        let meta = req
            .ctx
            .path()
            .map(|path| (grouping(path), query_hash(path, req.payload.bytes())));

        let response = next.run(req).await?;

        let Some((group, hash)) = meta else {
            return Ok(response);
        };

        // Both values are guaranteed-valid header material: labels are
        // lowercase ASCII kebab-case, hashes are hex.
        let response = match group {
            Some(group) => response.with_header(GROUPING_HEADER, group.as_ref()),
            None => response,
        };

        Ok(response.with_header(QUERY_HASH_HEADER, hash))
    }

    async fn intercept_streaming(
        &self,
        req: StreamRequest,
        inbound: PayloadStream,
        next: NextStream<'_>,
    ) -> Result<StreamResponse, ConnectError> {
        let group = req.ctx.path().and_then(grouping);

        let response = next.run(req, inbound).await?;

        Ok(match group {
            Some(group) => response.with_header(GROUPING_HEADER, group.as_ref()),
            None => response,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_services_use_curated_labels() {
        assert_eq!(
            grouping("/routers.api.match.v1.MatchService/Match").as_deref(),
            Some("path-analysis")
        );
        assert_eq!(
            grouping("/routers.api.match.v1.MatchService/Snap").as_deref(),
            Some("path-analysis")
        );
        assert_eq!(
            grouping("/routers.api.optimise.v1.OptimiseService/Route").as_deref(),
            Some("route-optimisation")
        );
        assert_eq!(
            grouping("/routers.api.scan.v1.ScanService/Point").as_deref(),
            Some("proximity-scan")
        );
        assert_eq!(
            grouping("/routers.api.timezone.v1.TimezoneService/GetFromPoint").as_deref(),
            Some("timezone-lookup")
        );
    }

    #[test]
    fn unknown_services_derive_a_kebab_case_label() {
        assert_eq!(
            grouping("/example.v1.PathAnalysisService/Run").as_deref(),
            Some("path-analysis")
        );
        assert_eq!(grouping("/example.v1.Echo/Run").as_deref(), Some("echo"));
    }

    #[test]
    fn underivable_labels_are_none() {
        assert_eq!(grouping(""), None);
        assert_eq!(grouping("/example.v1.Service/Run"), None);
    }

    #[test]
    fn hash_is_deterministic() {
        let path = "/routers.api.match.v1.MatchService/Match";
        assert_eq!(query_hash(path, b"payload"), query_hash(path, b"payload"));
    }

    #[test]
    fn hash_distinguishes_payloads_and_paths() {
        let path = "/routers.api.match.v1.MatchService/Match";
        let other = "/routers.api.match.v1.MatchService/Snap";

        assert_ne!(query_hash(path, b"a"), query_hash(path, b"b"));
        assert_ne!(query_hash(path, b"a"), query_hash(other, b"a"));
    }

    #[test]
    fn hash_is_hex_encoded_sha256() {
        let hash = query_hash("/example.v1.Echo/Run", b"payload");

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
