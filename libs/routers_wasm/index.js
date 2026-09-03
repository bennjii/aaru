// Flat entry point for @routers-org/wasm.
//
// jco maps the exported `router` WIT interface to one namespace object, so the
// raw transpiled module only offers `import { router } from ...`. This file
// hoists the interface's members to top-level exports so consumers can write
// `import { Engine } from "@routers-org/wasm"`. The namespace is kept too.
import { router } from "./dist/transpiled/routers_wasm.js";

export const Engine = router.Engine;
export { router };
