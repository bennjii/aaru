//! A WebAssembly **component** for the Routers engine.
//!
//! A consumer loads map **shards** on demand and matches / routes / queries over
//! whatever is resident, entirely in-process — evicting shards to bound memory
//! while panning a map. The interface is defined in
//! [`wit/world.wit`](../wit/world.wit) and consumed as a component — typed from
//! JS via `jco`, or from any Wasmtime host — so there's no hand-written
//! marshalling.
//!
//! ```js
//! import { router } from "./transpiled/routers_wasm.component.js";
//! const engine = new router.Engine();
//! // As the viewport moves, load what it covers and evict the rest:
//! const id = router.Engine.shardOf(center);
//! engine.loadShard(id, await fetchShard(id));   // + shardNeighbours(id)
//! engine.match(trace, { searchDistance: 50, reachDistance: 5000 });
//! engine.route(start, end);   // least-cost path across loaded shards
//! ```
//!
//! The [`engine`] module holds the pure core (host-testable, incl. a cross-shard
//! match); the `bindings` module (wasm only) maps the WIT interface onto it.

// wit-bindgen's component export glue contains `unsafe`; the workspace denies
// `unsafe_code` by default, so allow it for this binding crate only.
#![allow(unsafe_code)]

pub mod engine;

#[cfg(target_arch = "wasm32")]
mod bindings;
