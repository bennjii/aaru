//! Matches the Sydney trace against the routing network.
//!
//! The codec is selected at compile time: the default build ingests the OSM
//! PBF fixture, and the `overture` feature switches to the Overture
//! GeoParquet fixture. The matching pipeline itself is untouched by the
//! swap — both networks implement the same `routers_network` traits.
//!
//! ```sh
//! cargo run --release --example map_match
//! cargo run --release --features overture --example map_match
//! ```

use std::sync::Arc;
use std::time::Instant;

use geo::{LineString, Point};
use routers::uom::si::f64::Length;
use routers::uom::si::length::kilometer;
use routers::{Match, MatchOptions, primitives::PredicateCache};
use wkt::TryFromWkt;

use routers_fixtures::SYNDEY_TRIP;

/// The OSM codec: the default implementation.
#[cfg(not(feature = "overture"))]
mod source {
    use routers::codec::osm;
    use routers_fixtures::{SYDNEY, fixture_path};

    pub type Net = osm::OsmNetwork;

    pub fn network() -> Net {
        Net::from_pbf(&fixture_path(SYDNEY)).expect("network must be created")
    }
}

/// The Overture codec: swapped in by the `overture` feature.
#[cfg(feature = "overture")]
mod source {
    use routers::codec::overture;
    use routers_fixtures::{SYDNEY_OVERTURE, fixture_path};

    pub type Net = overture::OvertureNetwork;

    pub fn network() -> Net {
        Net::from_geoparquet(&fixture_path(SYDNEY_OVERTURE)).expect("network must be created")
    }
}

use source::Net;

fn main() {
    let coordinates: LineString<f64> =
        LineString::try_from_wkt_str(SYNDEY_TRIP).expect("must parse");

    let now = Instant::now();
    let network = source::network();
    println!("Ingest took: {:?}", now.elapsed());

    // Consecutive trace positions sit further apart (by network distance)
    // than the default one-kilometre transition reach.
    let cache = Arc::new(PredicateCache::<Net>::with_reach_distance(Length::new::<
        kilometer,
    >(2.0)));

    let now = Instant::now();
    let route = network
        .r#match(coordinates, MatchOptions::new().with_cache(cache))
        .expect("match must complete successfully");

    let linestring = route
        .interpolated
        .iter()
        .map(|v| Point(v.point))
        .collect::<LineString<_>>();

    println!("Matched Route: {linestring:?}");
    println!("Time taken: {:?}", now.elapsed());
}
