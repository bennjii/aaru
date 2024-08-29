use tokio::time::Instant;

use aaru::server::route::router_service::router_client::RouterClient;
use aaru::server::route::router_service::{Coordinate, Costing, RouteRequest};

#[tokio::main]
async fn main() {
    let mut router = RouterClient::connect("http://[::1]:9001").await
        .expect("Couldnt start channel.");

    let timer = Instant::now();

    let request = tonic::Request::new(RouteRequest {
        costing_method: i32::from(Costing::Car),
        end: Some(Coordinate {
            latitude: -33.883572,
            longitude: 151.180025
        }),
        start: Some(Coordinate {
            latitude: -33.890029,
            longitude: 151.201438
        })
    });

    let response = router.route(request).await
        .expect("Couldnt find point, failure.");

    println!("Got Coordinate: {:?}", response.get_ref());
    println!("Time elapsed: {:?}", timer.elapsed()); // Should be about 500µs-2ms
}