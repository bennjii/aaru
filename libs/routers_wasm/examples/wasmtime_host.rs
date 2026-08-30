//! Run the routing component from a native Wasmtime host: load every shard in a
//! directory, then match.
//!
//!   just wasm-shards   # writes libs/routers_wasm/dist/shards/*.shard.rt
//!   cargo run -p routers_wasm --example wasmtime_host -- \
//!     libs/routers_wasm/dist/routers_wasm.component.wasm libs/routers_wasm/dist/shards

use std::fs;

use wasmtime::component::{Component, Linker, ResourceAny};
use wasmtime::{Engine, Store};

mod bindings {
    wasmtime::component::bindgen!({ world: "routers", path: "wit" });
}

use bindings::exports::routers::routing::router::{Coordinate, MatchOptions};

fn coord(longitude: f64, latitude: f64) -> Coordinate {
    Coordinate {
        latitude,
        longitude,
    }
}

fn main() -> wasmtime::Result<()> {
    let mut args = std::env::args().skip(1);
    let component_path = args
        .next()
        .unwrap_or_else(|| "libs/routers_wasm/dist/routers_wasm.component.wasm".to_string());
    let shard_dir = args
        .next()
        .unwrap_or_else(|| "libs/routers_wasm/dist/shards".to_string());

    let engine = Engine::default();
    let component = Component::from_file(&engine, &component_path)?;
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());

    let world = bindings::Routers::instantiate(&mut store, &component, &linker)?;
    let router = world.routers_routing_router();
    let handle: ResourceAny = router.engine().call_constructor(&mut store)?;

    // Load every `{id}.shard.rt` in the directory.
    let mut loaded = 0;
    for entry in fs::read_dir(&shard_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rt") {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_suffix(".shard"))
            .unwrap_or_default()
            .to_string();
        let bytes = fs::read(&path)?;
        router
            .engine()
            .call_load_shard(&mut store, handle, &id, &bytes)?
            .map_err(|e| wasmtime::Error::msg(format!("load-shard {id}: {e:?}")))?;
        loaded += 1;
    }
    println!(
        "loaded {loaded} shards: {:?}",
        router.engine().call_loaded_shards(&mut store, handle)?
    );

    // Match a trace over the loaded shards.
    let trace = [
        (151.194792, -33.88538),
        (151.192818, -33.888159),
        (151.188054, -33.891864),
        (151.182733, -33.89425),
    ]
    .map(|(lon, lat)| coord(lon, lat));

    let options = MatchOptions {
        search_distance: Some(50.0),
        reach_distance: Some(5000.0),
        optimise_for: None,
        transport: None,
    };
    let matched = router
        .engine()
        .call_match(&mut store, handle, &trace, options)?
        .map_err(|e| wasmtime::Error::msg(format!("match failed: {e:?}")))?;
    println!("match: {} discretized points", matched.discretized.len());

    Ok(())
}
