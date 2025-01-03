use aaru::server::route::router_service::router_service_client::RouterServiceClient;
use aaru::server::route::router_service::{
    ClosestSnappedPointRequest, Coordinate, Costing, RouteRequest,
};

use tokio;
use tokio::time::Instant;
use tonic;
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://[::1]:9001").connect().await?;
    let mut client = RouterServiceClient::new(channel);

    let request = tonic::Request::new(ClosestSnappedPointRequest {
        point: Some(Coordinate {
            latitude: -33.850842,
            longitude: 151.210193,
        }),
        search_radius: 100.0,
    });

    let start = Instant::now();
    let response = client.closest_snapped_point(request).await?;
    println!("Snapped point: {:?}", response);
    let elapsed = start.elapsed();
    println!("In: {}us ({}ms)", elapsed.as_micros(), elapsed.as_millis());

    let route = tonic::Request::new(RouteRequest {
        start: Some(Coordinate {
            longitude: 151.17967159998506,
            latitude: -33.88689110000605,
        }),
        end: Some(Coordinate {
            longitude: 151.18187959999403,
            latitude: -33.88566269999858,
        }),
        costing_method: i32::from(Costing::Car),
    });

    let start = Instant::now();
    let response = client.route(route).await?;
    println!("Routed points: {:?}", response);
    let elapsed = start.elapsed();
    println!("In: {}us ({}ms)", elapsed.as_micros(), elapsed.as_millis());

    Ok(())
}
