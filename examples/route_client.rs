use tokio::time::Instant;
use tonic::transport::Channel;
use aaru::server::route::router_service::{ClosestSnappedPointRequest, Coordinate};
use aaru::server::route::router_service::router_client::RouterClient;
use tonic;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://[::1]:9001").connect().await?;
    let mut client = RouterClient::new(channel);

    let request = tonic::Request::new(ClosestSnappedPointRequest {
        point: Some(Coordinate { latitude: -33.850842, longitude: 151.210193 }),
        distance: 70,
    });

    let start = Instant::now();
    let response = client.closest_snapped_point(request).await?;
    println!("Snapped point: {:?}", response);
    let elapsed = start.elapsed();
    print!("In: {}us ({}ms)", elapsed.as_micros(), elapsed.as_millis());

    Ok(())
}