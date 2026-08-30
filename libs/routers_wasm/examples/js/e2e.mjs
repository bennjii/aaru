// Navigation E2E for the transpiled component. Run after:
//   just wasm-shards      # writes libs/routers_wasm/dist/shards/*.shard.rt
//   just wasm-transpile   # writes libs/routers_wasm/dist/transpiled/
//   node libs/routers_wasm/js/e2e.mjs
//
// Simulates panning a map along a route: at each viewport it loads the covering
// shard + neighbours (a browser would `fetch()` these), evicts what's no longer
// in view, then matches over whatever is resident — proving bounded-memory,
// pan-and-match. Same code a browser runs (the component is portable).

import { readFileSync } from "node:fs";
import { router } from "../dist/transpiled/routers_wasm.component.js";

const SHARD_DIR = "libs/routers_wasm/dist/shards";
const at = (lng, lat) => ({ latitude: lat, longitude: lng });

const engine = new router.Engine();
const resident = new Set();

// Load the shard covering `center` plus its neighbours; evict everything else.
// In a browser, `readFileSync` is `await fetch(url)`.
function recenter(center) {
  const needed = new Set([
    router.Engine.shardOf(center),
    ...router.Engine.shardNeighbours(router.Engine.shardOf(center)),
  ]);
  for (const id of needed) {
    if (resident.has(id)) continue;
    try {
      engine.loadShard(id, new Uint8Array(readFileSync(`${SHARD_DIR}/${id}.shard.rt`)));
      resident.add(id);
    } catch {
      /* no shard file for this cell (outside the extract) */
    }
  }
  for (const id of [...resident]) {
    if (!needed.has(id)) {
      engine.unloadShard(id);
      resident.delete(id);
    }
  }
}

// Pan along the route.
const waypoints = [
  [151.194792, -33.88538],
  [151.188054, -33.891864],
  [151.182733, -33.89425],
];
for (const [lng, lat] of waypoints) {
  recenter(at(lng, lat));
  console.log(`@ ${lat},${lng}  resident shards: ${engine.loadedShards().length}`);
}

// Match a trace over whatever is currently loaded.
const trace = waypoints.map(([lng, lat]) => at(lng, lat));
const matched = engine.match(trace, { searchDistance: 50, reachDistance: 5000 });
console.log(`match: ${matched.discretized.length} discretized points`);

if (matched.discretized.length === 0) {
  console.error("empty match");
  process.exit(1);
}
