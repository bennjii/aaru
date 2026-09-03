// Flat entry point: hoists the `router` interface's members to top-level exports.
import { router } from "./dist/transpiled/routers_wasm.js";

export const Engine = router.Engine;
export { router };
