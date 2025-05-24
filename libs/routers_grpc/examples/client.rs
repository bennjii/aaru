use geo::{LineString, coord, wkt};
use routers_fixtures::LAX_LYNWOOD_TRIP;
use routers_grpc::r#match::*;
use std::fmt::Error;
use std::fs::File;
use std::io::Write;
use tokio;
use tokio::time::Instant;
use tonic;
use tonic::Request;
use tonic::transport::Channel;
use wkt::{ToWkt, TryFromWkt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://[::1]:9001").connect().await?;
    let mut client = MatchServiceClient::new(channel);

    let route = Request::new(MatchRequest {
        data: LineString::try_from_wkt_str(LAX_LYNWOOD_TRIP)?.into(),
        ..Default::default()
    });

    let response = client.r#match(route).await?;

    let linestring = response.get_ref().snapped();
    let interpolated = response.get_ref().interpolated();

    println!("Routed points: {}", linestring.into().wkt_string());
    println!("Interpolated path: {}", interpolated.into().wkt_string());

    let path = "routed.wkt";
    let mut output = File::create(path).unwrap();
    write!(output, "{}", interpolated.into().wkt_string()).expect("must write");

    Ok(())
}
