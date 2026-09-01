// Publish gate for the npm package: the transpiled module must load, its wasm
// core must instantiate, and the engine must answer without any shards. Run
// after `just wasm-transpile` (and `npm install` for the WASI shim):
//   node libs/routers_wasm/examples/js/smoke.mjs
// The full navigation E2E (shard loading + matching) is e2e.mjs.

import { router } from "../../dist/transpiled/routers_wasm.js";

const sydney = { latitude: -33.89, longitude: 151.19 };

const id = router.Engine.shardOf(sydney);
const neighbours = router.Engine.shardNeighbours(id);
if (!id || neighbours.length === 0) {
  console.error(`bad shard cover: id=${id}, neighbours=${neighbours.length}`);
  process.exit(1);
}

const engine = new router.Engine();
if (engine.loadedShards().length !== 0) {
  console.error("fresh engine reports resident shards");
  process.exit(1);
}

console.log(`smoke ok: ${sydney.latitude},${sydney.longitude} -> shard ${id} (+${neighbours.length} neighbours)`);
