### `codec`

Codec is a part of the `routers` project. 

It aims to provide the tooling required to efficiently ingest 
data from various geo-spacial formats into a `routers` graph.

There are modules within this crate which describe the process
to perform this ingestion efficiently, one per source format:

- [`crate::osm`] — OpenStreetMap `.osm.pbf` extracts
- [`crate::overture`] — Overture Maps transportation GeoParquet

Each module is self-contained — its own parsers, elements and network
type — and every network implements the same `routers_network` traits,
so consumers generic over `N: Network` accept either. At a glance:

|                | [`crate::osm`]                      | [`crate::overture`]                          |
|----------------|-------------------------------------|----------------------------------------------|
| Source format  | `.osm.pbf` protobuf blobs           | GeoParquet (`segment` / `connector`)         |
| Graph nodes    | `Node` elements                     | `connector` features                         |
| Graph edges    | `Way` references, windowed by two   | `segment` chains, split at connectors        |
| Identifiers    | numeric element ids                 | GERS strings, interned to `i64`              |
| One-way        | `oneway` / `junction` tags          | `access_restrictions` + `when.heading`       |
| Lane data      | `lanes` tags                        | absent from the schema                       |
| Network type   | `OsmNetwork`                        | `OvertureNetwork`                            |

Routing weights use the same class tiers in both modules, so costs are
comparable across networks built from either source.
