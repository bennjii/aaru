Defines parser logic and graph assembly for [Overture Maps](https://overturemaps.org)
transportation data.

### Model

Overture distributes its transportation theme as **GeoParquet**, partitioned
into two feature types:

```text
theme=transportation
  ├─┬─ type=connector          - Graph nodes
  │ └── { id, geometry: Point }
  │
  └─┬─ type=segment            - Edge chains
    ├── { id, geometry: LineString }
    ├── subtype (road | rail | water)
    ├── class + subclass       - See [`RoadClass`]
    ├── connectors[]           - { connector_id, at: 0..=1 }
    ├── speed_limits[]         - See [`SpeedLimit`]
    └── access_restrictions[]  - See [`AccessRestriction`]
```

A **connector** is a bare junction point. A **segment** references two or
more connectors, each at a fractional position (`at`) along its linestring.
Two segments are joined *if and only if* they share a `connector_id` —
geometric overlap alone does not connect them.

[`OvertureNetwork::from_elements`] splits each segment at its sorted
connector positions. The linestring's interior vertices between two
connectors are materialised as synthetic nodes, so the resulting edge chain
follows the road shape rather than cutting the corner between junctions.

### Semantics

- **Directionality** is expressed through access rules, not a dedicated
  flag: an [`AccessRestriction`] with `access_type: denied` scoped to a
  [`Heading`] (`forward` / `backward`, relative to linestring order) closes
  that direction of travel, yielding a one-way segment.
- **Ramps and slip roads** are not distinct classes; they are the parent
  [`RoadClass`] plus `subclass: link`, carried on [`Segment`] and folded
  into the routing weight.
- **Lane counts do not exist** in the schema, so
  [`OvertureEdgeMetadata::lane_count`] is always `None`.
- **Identifiers** are GERS strings; an [`Interner`] maps them to dense
  `i64`-backed [`OvertureEntryId`]s during ingestion, and only those
  integers are stored in the graph.

### Usage

Reading GeoParquet requires the `overture` cargo feature, which gates the
Arrow/Parquet reader stack:

```rust,ignore
let network = OvertureNetwork::from_geoparquet(&path)?;

// Or, with an `.rt` cache alongside:
let network = OvertureNetwork::from_geoparquet_and_save(&source, &cache)?;
```

The graph itself, its parsers and its `.rt` (de)serialisation compile
unconditionally — a prebuilt cache loads via [`OvertureNetwork::from_bytes`]
on any target, including WASM.

[`OvertureNetwork`] implements the `routers_network` traits ([`DataPlane`],
[`Discovery`], [`Scan`], [`Route`]), so it drops into any consumer generic
over `N: Network`.

### Not yet modelled

- `between` linear-referenced attribute scoping (segment-level attributes
  currently apply to every sub-edge, including synthetic shape hops);
  [`Between`] carries the range for this follow-up.
- `prohibited_transitions` turn restrictions.
- Remote (S3/Azure) GeoParquet fetching.
- `rail` and `water` subtypes — only `road` segments enter the graph.

[`OvertureNetwork`]: crate::overture::OvertureNetwork
[`OvertureNetwork::from_elements`]: crate::overture::OvertureNetwork::from_elements
[`OvertureNetwork::from_bytes`]: crate::overture::OvertureNetwork::from_bytes
[`OvertureEdgeMetadata::lane_count`]: crate::overture::OvertureEdgeMetadata::lane_count
[`OvertureEntryId`]: crate::overture::OvertureEntryId
[`Interner`]: crate::overture::Interner
[`Segment`]: crate::overture::Segment
[`RoadClass`]: crate::overture::RoadClass
[`SpeedLimit`]: crate::overture::SpeedLimit
[`AccessRestriction`]: crate::overture::AccessRestriction
[`Heading`]: crate::overture::Heading
[`Between`]: crate::overture::Between
[`DataPlane`]: routers_network::DataPlane
[`Discovery`]: routers_network::Discovery
[`Scan`]: routers_network::Scan
[`Route`]: routers_network::Route
