# Sizing inputs. Every number is measurable, not a preference — change a
# calibration only with a benchmark behind it. The reasoning behind the
# topology lives in ARCHITECTURE.md; this file states the numbers.

variable "throughput_target_eps" {
  description = "Raw events per second to absorb."
  type        = number

  validation {
    condition     = var.throughput_target_eps > 0
    error_message = "throughput_target_eps must be positive."
  }
}

variable "shards" {
  description = <<-EOT
    Road-bearing geohash shards to deploy, at `shard_precision`, as emitted by
    `cargo run -p routers_shard --bin generate-shards`.

    Never compute this as 32^precision. Precision 6 is 1.07e9 cells globally and
    almost all are empty; each shard listed costs a matcher and an orchestrator
    whether or not it sees traffic.
  EOT
  type        = list(string)

  validation {
    condition     = length(var.shards) > 0
    error_message = "At least one shard is required."
  }

  validation {
    condition     = length(var.shards) == length(distinct(var.shards))
    error_message = "shards must not contain duplicates; each shard owns exactly one matcher."
  }
}

variable "shard_precision" {
  description = "Geohash precision of `shards`, passed to the matcher's PRECISION. Must match the `.shard.rt` files, or matchers load the wrong extent."
  type        = number

  validation {
    condition     = var.shard_precision >= 1 && var.shard_precision <= 12 && floor(var.shard_precision) == var.shard_precision
    error_message = "shard_precision must be a whole number in 1..12."
  }

  # Otherwise the subject split reads past the end of a shard string, and the
  # failure surfaces as an out-of-range substr rather than as the mismatch it is.
  validation {
    condition     = alltrue([for s in var.shards : length(s) == var.shard_precision])
    error_message = "Every shard must be exactly shard_precision characters long."
  }
}

variable "cell_precision" {
  description = <<-EOT
    Geohash prefix length defining a cell: what one historian subscribes to
    (`position.<cell>.*`) and what one Helm release covers. Prefixes nest, so
    shard `9q8yyz` sits in cell `9q`.

    Not a NATS partition. Moving this dial costs no broker capacity.

    Precision 1 is 45 degrees square, and CONUS is `9`, `c`, `d` and `f`.
    Precision 2 divides that by 8 in longitude and 4 in latitude.
  EOT
  type        = number
  default     = 2

  validation {
    condition     = var.cell_precision >= 1 && var.cell_precision <= 12 && floor(var.cell_precision) == var.cell_precision
    error_message = "cell_precision must be a whole number in 1..12."
  }

  validation {
    condition     = var.cell_precision < var.shard_precision
    error_message = "cell_precision must be below shard_precision, or the shard token of the subject would be empty and match nothing."
  }
}

variable "subject_prefix" {
  description = <<-EOT
    Root of the two-phase subject scheme: `<prefix>.<verb>.<cell>.<rest>`.

    A NATS wildcard matches exactly one token, so splitting the geohash is what
    makes `position.<cell>.*` address a whole cell. In a single token neither
    that nor a per-cell historian is expressible.
  EOT
  type        = string
  default     = "events"
}

variable "coverage_cells" {
  description = "Optional allowlist of cell prefixes for the service region. Catches a shard list generated over the wrong extent before it becomes a fleet of idle matchers."
  type        = list(string)
  default     = []
}

variable "shard_fanout_per_precision" {
  description = <<-EOT
    Growth in road-bearing shard count per precision level, used only to
    recommend a precision.

    Geohash grows 32x per level, but populated cells grow far slower: a road
    network is close to one-dimensional inside a two-dimensional cell, so
    subdividing mostly yields empty children. Recalibrate by generating shards
    at two adjacent precisions and taking the ratio.
  EOT
  type        = number
  default     = 8

  validation {
    condition     = var.shard_fanout_per_precision > 1
    error_message = "shard_fanout_per_precision must exceed 1, or raising precision would add no capacity."
  }
}

variable "headroom_ratio" {
  description = "Spare capacity above the target. Absorbs diurnal peaks and the replay burst after a matcher restart."
  type        = number
  default     = 0.25

  validation {
    condition     = var.headroom_ratio >= 0 && var.headroom_ratio < 5
    error_message = "headroom_ratio must be in [0, 5)."
  }
}

variable "vertical_profile" {
  description = "Which `profiles` entry to deploy."
  type        = string
  default     = "standard"
}

variable "profiles" {
  description = <<-EOT
    Vertical scaling steps. `shard_eps` is the measured sustained input rate for
    one shard's matcher+orchestrator pair, and converts a shard count into a
    throughput.

    Anchored on the chart's shipped defaults: 32 orchestrator workers hold ~6k
    evt/s per shard against a ~5ms mean Valkey fetch.
  EOT
  type = map(object({
    matcher_workers         = number
    matcher_cpu_millis      = number
    matcher_memory_mib      = number
    orchestrator_workers    = number
    orchestrator_cpu_millis = number
    orchestrator_memory_mib = number
    historian_cpu_millis    = number
    historian_memory_mib    = number
    shard_eps               = number
  }))

  default = {
    small = {
      matcher_workers         = 5
      matcher_cpu_millis      = 1000
      matcher_memory_mib      = 3072
      orchestrator_workers    = 8
      orchestrator_cpu_millis = 500
      orchestrator_memory_mib = 1024
      historian_cpu_millis    = 250
      historian_memory_mib    = 256
      shard_eps               = 1500
    }

    # The calibration anchor.
    standard = {
      matcher_workers         = 10
      matcher_cpu_millis      = 2000
      matcher_memory_mib      = 3072
      orchestrator_workers    = 32
      orchestrator_cpu_millis = 1000
      orchestrator_memory_mib = 1024
      historian_cpu_millis    = 500
      historian_memory_mib    = 256
      shard_eps               = 6000
    }

    # Fewer, fatter shards. Fewer Deployments is its own win: ~3000 of them
    # makes rollouts an API-server problem.
    large = {
      matcher_workers         = 24
      matcher_cpu_millis      = 6000
      matcher_memory_mib      = 6144
      orchestrator_workers    = 96
      orchestrator_cpu_millis = 3000
      orchestrator_memory_mib = 2048
      historian_cpu_millis    = 1000
      historian_memory_mib    = 512
      shard_eps               = 16000
    }
  }
}

# --- Nodes ------------------------------------------------------------------

variable "machines" {
  description = "Machine shape per node pool. `vcpu` and `memory_gib` must match `machine_type`; the model derives allocatable capacity from them."
  type = map(object({
    machine_type = string
    vcpu         = number
    memory_gib   = number
  }))
}

variable "max_pods_per_node" {
  description = "Pods per node. The node's pod CIDR is sized from this at pool creation, so raising it later needs a new pool."
  type        = number
  default     = 110

  validation {
    condition     = var.max_pods_per_node >= 8 && var.max_pods_per_node <= 256
    error_message = "max_pods_per_node must be in 8..256; GKE Standard allows at most 110."
  }
}

variable "daemonset_cpu_millis" {
  description = "Per-node CPU claimed by DaemonSets before any workload: kube-proxy, gke-metadata-server, netd, the logging and metrics agents, the GCS FUSE CSI node driver."
  type        = number
  default     = 600
}

variable "daemonset_memory_mib" {
  description = "Per-node memory claimed by the same DaemonSets."
  type        = number
  default     = 1200
}

variable "daemonset_pods_per_node" {
  description = "DaemonSet pods per node, deducted from the schedulable pod budget."
  type        = number
  default     = 10
}

# --- NATS -------------------------------------------------------------------

variable "nats_hops" {
  description = "Message deliveries per input event. A raw event reaches the orchestrator and the historian (2), the orchestrator publishes a match context (3), and the matcher publishes a result back (4)."
  type        = number
  default     = 4
}

variable "nats_min_replicas" {
  description = <<-EOT
    Availability floor for the cluster, so a node loss leaves two servers
    carrying the subject space.

    Not a quorum: core NATS elects nothing and has no RAFT, which is also why
    the cluster size is not rounded to an odd number.
  EOT
  type        = number
  default     = 3

  validation {
    condition     = var.nats_min_replicas >= 1
    error_message = "nats_min_replicas must be at least 1."
  }
}

variable "nats_msgs_per_server" {
  description = <<-EOT
    Deliveries per second one NATS server sustains, for postcard payloads of
    100B-10KiB.

    The least trustworthy number in this model. Every official NATS figure is a
    single server over loopback on a laptop — the project publishes no clustered
    benchmarks — and real networked hardware runs well under them. This default
    is the published 4M aggregate for 1 publisher to 4 subscribers at 128B, with
    a 4x haircut.

    Measure it with `nats bench` on the real node shape before trusting the
    server count it produces.
    https://docs.nats.io/using-nats/nats-tools/nats_cli/natsbench
  EOT
  type        = number
  default     = 1000000

  validation {
    condition     = var.nats_msgs_per_server > 0
    error_message = "nats_msgs_per_server must be positive."
  }
}

variable "nats_cpu_millis" {
  description = "CPU request per NATS server. Core NATS is a routing loop, so this buys network stack rather than compute."
  type        = number
  default     = 4000
}

variable "nats_memory_mib" {
  description = <<-EOT
    Memory request per NATS server. Connection buffers and slow-consumer queues
    only, because JetStream is off.

    Off deliberately: the official benchmarks put a single stream between 2.5x
    and 24x under core NATS on the same hardware, and the durability it offers
    is already held in Valkey. See ARCHITECTURE.md.
  EOT
  type        = number
  default     = 8192
}

# --- Valkey -----------------------------------------------------------------

variable "valkey_ops_per_primary" {
  description = <<-EOT
    Commands per second one Valkey primary sustains.

    The figure measured in Valkey's 1-billion-RPS cluster run: 2000 primaries on
    r7g.2xlarge (8 vCPU) with 6 io-threads, ~500k ops/s each, growing almost
    linearly with primary count. https://valkey.io/blog/1-billion-rps/

    Single-node benchmarks reach ~1.19M ops/s, but with no replication and no
    cluster bus, so that number does not describe a fleet member.

    Not to be confused with the repo's "valkey is single-threaded" comments:
    command execution is still one thread, but Valkey 8 moved I/O parsing and
    writing onto threads.
  EOT
  type        = number
  default     = 500000

  validation {
    condition     = var.valkey_ops_per_primary > 0
    error_message = "valkey_ops_per_primary must be positive."
  }
}

variable "valkey_io_threads" {
  description = "Valkey `io-threads`, the vertical lever. The 1B RPS run used 6 on an 8-core node; sizing them to every core starves the interrupt handlers and throughput drops."
  type        = number
  default     = 6

  validation {
    condition     = var.valkey_io_threads >= 1
    error_message = "valkey_io_threads must be at least 1."
  }
}

variable "valkey_ops_per_event" {
  description = <<-EOT
    Commands per input event: one XREVRANGE read by the orchestrator, one
    pipelined XADD by the historian.

    The read is the cheapest thing to remove. Vehicles are hash-pinned to a
    worker and stay in one orchestrator while in a shard, so a per-vehicle cache
    would drop this to ~1 and double the per-primary event ceiling — a better
    return than any topology change here.
  EOT
  type        = number
  default     = 2
}

variable "valkey_replicas_per_primary" {
  description = "Read replicas per primary, for failover only. The pipeline never reads a replica: the orchestrator's fetch must see the historian's latest write."
  type        = number
  default     = 1
}

variable "valkey_cpu_millis" {
  description = "CPU request per Valkey pod. No CPU limit is set: a CFS cap would freeze the command loop in 100ms windows and queue every in-flight command behind it."
  type        = number
  default     = 2000
}

variable "valkey_memory_mib" {
  description = "Memory request per Valkey pod. One stream per vehicle trimmed to HISTORY entries, so bounded by concurrent vehicles rather than event rate."
  type        = number
  default     = 4096
}

variable "valkey_client_mode" {
  description = <<-EOT
    How clients address the fleet.

    `pooled-hash` holds one multiplexed connection per primary and places a
    vehicle by rendezvous hash over the URLs. No Redis Cluster protocol is
    involved: no slot map, no `MOVED`, no cluster bus.

    `single` reaches one primary only, and caps the deployment at its
    throughput.
  EOT
  type        = string
  default     = "pooled-hash"

  validation {
    condition     = contains(["single", "pooled-hash"], var.valkey_client_mode)
    error_message = "valkey_client_mode must be 'single' or 'pooled-hash'."
  }
}

# --- Historian --------------------------------------------------------------

variable "historian_mode" {
  description = <<-EOT
    How historian work is divided across subjects.

    `per-cell` subscribes one historian to `position.<cell>.*`, which the
    two-phase subject makes possible. One pod per cell rather than per shard.

    `per-shard` subscribes to each exact shard subject. Use it when one pod
    cannot keep up with a cell and a queue group is unavailable.

    `global` runs a fixed count on `position.>`. Correct only at one replica
    unless `historian_queue_group` is set.
  EOT
  type        = string
  default     = "per-cell"

  validation {
    condition     = contains(["per-cell", "per-shard", "global"], var.historian_mode)
    error_message = "historian_mode must be 'per-cell', 'per-shard' or 'global'."
  }
}

variable "global_historian_replicas" {
  description = "Replicas when historian_mode is 'global'. Above 1 duplicates every write unless historian_queue_group is set."
  type        = number
  default     = 1
}

variable "historian_eps_per_pod" {
  description = "Events per second one historian sustains. It deserialises each event and appends it in a batched pipeline, so this is well above a shard's rate but below a large cell's."
  type        = number
  default     = 150000
}

variable "historian_queue_group" {
  description = <<-EOT
    Whether historians share a subject through a NATS queue group. Members split
    the deliveries instead of each receiving all of them, so replicas follow a
    cell's load and cells stay coarse.

    Without it a cell needs one historian able to absorb its whole write rate,
    and the only lever is a finer `cell_precision` — which costs a pod and a
    Helm release per cell.

    `historian.rs` honours this through QUEUE_GROUP. Leave it off for an image
    built before that argument existed, where every replica would archive every
    event and the duplicates would evict real history under MAXLEN.
  EOT
  type        = bool
  default     = false
}

# --- Telemetry --------------------------------------------------------------

variable "spans_per_event" {
  description = "Spans emitted per input event across the three services: queue waits, the solve, the commit."
  type        = number
  default     = 8
}

variable "telemetry_sample_ratio" {
  description = <<-EOT
    Fraction of traces to export.

    Unsampled telemetry does not survive this pipeline: the BatchSpanProcessor
    drops the excess silently from a 16k queue, so metrics derived from it
    under-report by an unknown factor. Sampling turns that loss into a known
    constant. Use 1.0 only in development.
  EOT
  type        = number
  default     = 0.001

  validation {
    condition     = var.telemetry_sample_ratio > 0 && var.telemetry_sample_ratio <= 1
    error_message = "telemetry_sample_ratio must be in (0, 1]."
  }
}

variable "collector_spans_per_replica" {
  description = "Spans per second one otel-collector replica converts to spanmetrics. CPU-bound: the connector hashes every span into a histogram keyed by the promoted dimensions."
  type        = number
  default     = 50000
}

variable "collector_cpu_millis" {
  description = "CPU request per otel-collector replica."
  type        = number
  default     = 2000
}

variable "collector_memory_mib" {
  description = "Memory request per otel-collector replica. Holds spanmetrics cardinality between flushes."
  type        = number
  default     = 2048
}
