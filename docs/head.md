### `aaru`

Provides actionable area routing utilities for `OSM` data.

Each component is divided into its own module.

- `codec` Defines parser logic and iterators over `.osm.pbf` data

- `geo` Provides general purpose geo-utilities.

- `route` Yields routing utilities for fastest route, nearest node, etc.

- `tile` Provides a generic method to serve data as a slippy tile


#### Features
- `mmap` Enables file I/O using sys/memmap (pages)
- `tracing` Enables OpenTelemetry tracing for `server`