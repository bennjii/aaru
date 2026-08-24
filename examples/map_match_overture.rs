//! Matches the Sydney trace against both the OSM and Overture networks and
//! compares the resulting geometry.
//!
//! Run with:
//! ```sh
//! cargo run --release --features overture --example map_match_overture
//! ```

use std::sync::Arc;
use std::time::Instant;

use geo::{Closest, ClosestPoint, Coord, Distance, Haversine, LineString, Point};
use routers::{
    Match, MatchError, MatchOptions, candidate::Path, codec::osm::OsmNetwork,
    codec::overture::OvertureNetwork, primitives::PredicateCache, uom::si::f64::Length,
    uom::si::length::kilometer,
};
use routers_network::{Entry, Metadata, Network};
use wkt::TryFromWkt;

use routers_fixtures::{SYDNEY, SYDNEY_OVERTURE, SYNDEY_TRIP, fixture, fixture_path};

fn as_line<E: Entry, M: Metadata>(path: &Path<E, M>) -> LineString {
    path.iter().map(|v| Point(v.point)).collect()
}

/// Matches `coords`, returning the (discretized, interpolated) linestrings.
///
/// The trace has consecutive positions further apart (by network distance)
/// than the default one-kilometre transition reach, so both networks match
/// with a two-kilometre reach.
fn match_line<N>(
    net: &N,
    cache: &Arc<PredicateCache<N>>,
    coords: &LineString,
) -> Result<(LineString, LineString), MatchError>
where
    N: Network + Match<N>,
{
    let route = net.r#match(
        coords.clone(),
        MatchOptions::new().with_cache(cache.clone()),
    )?;
    Ok((as_line(&route.discretized), as_line(&route.interpolated)))
}

/// Distance in metres from `point` to the closest position on `line`.
fn deviation_to(line: &LineString, point: Point) -> f64 {
    match line.closest_point(&point) {
        Closest::Intersection(p) | Closest::SinglePoint(p) => Haversine.distance(point, p),
        Closest::Indeterminate => f64::NAN,
    }
}

/// Mean and max deviation of every vertex of `a` from the line `b`.
fn deviation_stats(a: &LineString, b: &LineString) -> (f64, f64) {
    let deviations: Vec<f64> = a.points().map(|p| deviation_to(b, p)).collect();
    let mean = deviations.iter().sum::<f64>() / deviations.len() as f64;
    let max = deviations.iter().copied().fold(0.0, f64::max);
    (mean, max)
}

fn main() {
    let coordinates: LineString<f64> =
        LineString::try_from_wkt_str(SYNDEY_TRIP).expect("must parse");

    let now = Instant::now();
    let osm = OsmNetwork::from_pbf(fixture!(SYDNEY)).expect("osm graph must be created");
    println!("OSM ingest took: {:?}", now.elapsed());

    let now = Instant::now();
    let overture = OvertureNetwork::from_geoparquet(&fixture_path(SYDNEY_OVERTURE))
        .expect("overture graph must be created");
    println!("Overture ingest took: {:?}", now.elapsed());

    let osm_cache = Arc::new(PredicateCache::<OsmNetwork>::with_reach_distance(
        Length::new::<kilometer>(2.0),
    ));
    let ovt_cache = Arc::new(PredicateCache::<OvertureNetwork>::with_reach_distance(
        Length::new::<kilometer>(2.0),
    ));

    // Report each network's ability to match the full trace, then compare
    // over the longest prefix both can match.
    let mut points: Vec<Coord> = coordinates.0.clone();
    let (osm_match, ovt_match) = loop {
        let prefix = LineString::from(points.clone());

        let now = Instant::now();
        let osm_result = match_line(&osm, &osm_cache, &prefix);
        println!(
            "osm match over {} points: {:?} ({})",
            points.len(),
            now.elapsed(),
            if osm_result.is_ok() { "ok" } else { "failed" }
        );

        let now = Instant::now();
        let ovt_result = match_line(&overture, &ovt_cache, &prefix);
        println!(
            "overture match over {} points: {:?} ({})",
            points.len(),
            now.elapsed(),
            if ovt_result.is_ok() { "ok" } else { "failed" }
        );

        match (osm_result, ovt_result) {
            (Ok(o), Ok(v)) => break (o, v),
            (osm_result, ovt_result) => {
                if let Err(e) = osm_result {
                    println!("osm: no match over {} points ({e:?})", points.len());
                }
                if let Err(e) = ovt_result {
                    println!("overture: no match over {} points ({e:?})", points.len());
                }
                points.pop();
                assert!(points.len() >= 2, "no common matchable prefix");
            }
        }
    };
    println!("\ncomparing over the first {} trace points", points.len());

    let (osm_discrete, osm_interp) = osm_match;
    let (ovt_discrete, ovt_interp) = ovt_match;

    // Discretized paths are one-to-one with the input trace: compare pointwise.
    let pointwise: Vec<f64> = osm_discrete
        .points()
        .zip(ovt_discrete.points())
        .map(|(a, b)| Haversine.distance(a, b))
        .collect();
    let mean = pointwise.iter().sum::<f64>() / pointwise.len() as f64;
    let max = pointwise.iter().copied().fold(0.0, f64::max);

    println!("\n== Discretized (matched position per input point) ==");
    println!("points: {}", pointwise.len());
    println!("pointwise deviation: mean {mean:.1}m, max {max:.1}m");
    for (i, d) in pointwise.iter().enumerate() {
        if *d > 10.0 {
            println!("  point {i:>2}: {d:.1}m apart");
        }
    }

    // Interpolated paths have different vertex counts: compare each vertex of
    // one line against the other line, in both directions.
    let (o2v_mean, o2v_max) = deviation_stats(&osm_interp, &ovt_interp);
    let (v2o_mean, v2o_max) = deviation_stats(&ovt_interp, &osm_interp);

    println!("\n== Interpolated (full recovered trip) ==");
    println!(
        "vertices: osm {}, overture {}",
        osm_interp.0.len(),
        ovt_interp.0.len()
    );
    println!("osm→overture deviation: mean {o2v_mean:.1}m, max {o2v_max:.1}m");
    println!("overture→osm deviation: mean {v2o_mean:.1}m, max {v2o_max:.1}m");
}
