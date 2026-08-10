//! End-to-end coverage for the [`QueryMetadata`] interceptor: a real server
//! with a stub handler, called through the generated client, must staple
//! the grouping and query-hash headers onto every response.

use buffa::MessageField;
use connectrpc::client::{ClientConfig, HttpClient};
use connectrpc::{ConnectRpcService, Response, Router, Server, handler_fn};
use core::net::SocketAddr;
use routers_rpc::meta::{GROUPING_HEADER, QUERY_HASH_HEADER, QueryMetadata};

use schema::connect::routers::api::timezone::v1::TimezoneServiceClient;
use schema::proto::routers::api::timezone::v1::{GetFromPointRequest, GetFromPointResponse};
use schema::proto::routers::model::v1::Coordinate;

async fn spawn_server() -> SocketAddr {
    let router = Router::new().route(
        "routers.api.timezone.v1.TimezoneService",
        "GetFromPoint",
        handler_fn(|_ctx, _req: GetFromPointRequest| async move {
            Ok(Response::new(GetFromPointResponse::default()))
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");

    let bound = Server::from_listener(listener);
    let addr = bound.local_addr().expect("bound listener has an address");

    let service = ConnectRpcService::new(router).with_interceptor(QueryMetadata);
    tokio::spawn(bound.serve_with_service(service));

    addr
}

fn client(addr: SocketAddr) -> TimezoneServiceClient<HttpClient> {
    let config = ClientConfig::new(format!("http://{addr}").parse().expect("valid uri"));
    TimezoneServiceClient::new(HttpClient::plaintext(), config)
}

fn request_at(longitude: f64, latitude: f64) -> GetFromPointRequest {
    GetFromPointRequest {
        coordinate: MessageField::some(Coordinate {
            longitude,
            latitude,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn responses_carry_query_metadata() {
    let client = client(spawn_server().await);

    let response = client
        .get_from_point(request_at(144.96, -37.81))
        .await
        .expect("rpc succeeds");

    let grouping = response
        .headers()
        .get(GROUPING_HEADER)
        .expect("grouping header stapled");
    assert_eq!(grouping, "timezone-lookup");

    let hash = response
        .headers()
        .get(QUERY_HASH_HEADER)
        .expect("query hash header stapled")
        .to_str()
        .expect("hash is ascii");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test(flavor = "multi_thread")]
async fn repeated_queries_hash_identically_and_distinct_queries_differ() {
    let client = client(spawn_server().await);

    let hash_of = |response: &connectrpc::client::UnaryResponse<_>| {
        response
            .headers()
            .get(QUERY_HASH_HEADER)
            .expect("query hash header stapled")
            .to_str()
            .expect("hash is ascii")
            .to_owned()
    };

    let first = client
        .get_from_point(request_at(144.96, -37.81))
        .await
        .expect("rpc succeeds");
    let repeat = client
        .get_from_point(request_at(144.96, -37.81))
        .await
        .expect("rpc succeeds");
    let other = client
        .get_from_point(request_at(151.21, -33.87))
        .await
        .expect("rpc succeeds");

    assert_eq!(hash_of(&first), hash_of(&repeat));
    assert_ne!(hash_of(&first), hash_of(&other));
}
