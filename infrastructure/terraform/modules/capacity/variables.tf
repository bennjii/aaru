# Sizing inputs. Every number is measurable, not a preference — change a
# calibration only with a benchmark behind it. Numbers this model cannot
# justify from a published figure or a measurement say so in their
# description, so an unmeasured default is never mistaken for a known one.

variable "throughput_target_eps" {
  description = "Raw events per second to absorb."
  type        = number

  validation {
    condition     = var.throughput_target_eps > 0
    error_message = "throughput_target_eps must be positive."
  }
}

variable "design_target_eps" {
  description = <<-EOT
    The rate the deployment is designed to reach, as opposed to the one it
    carries today.

    Only the wire-law quantities need to be right for it now: `partitions`,
    because producers hash into it, and `streams`, because revisions are
    stream sequences. Everything else — shards, replicas, fleet size, Valkey
    primaries — is a variable change away, so it is sized for
    `throughput_target_eps` and reported against this.
  EOT
  type        = number
  default     = 5000000

  validation {
    condition     = var.design_target_eps > 0
    error_message = "design_target_eps must be positive."
  }
}

variable "headroom_ratio" {
  description = "Spare capacity above the target. Absorbs diurnal peaks and the Restart burst after a matcher or orchestrator restart."
  type        = number
  default     = 0.25

  validation {
    condition     = var.headroom_ratio >= 0 && var.headroom_ratio < 5
    error_message = "headroom_ratio must be in [0, 5)."
  }
}

# --- Geography: the matcher half -------------------------------------------

variable "shards" {
  description = <<-EOT
    Road-bearing geohash shards to deploy, at `shard_precision`, as emitted by
    `cargo run -p routers_shard --bin generate-shards`.

    Never compute this as 32^precision. Precision 6 is 1.07e9 cells globally
    and almost all are empty; each shard listed costs at least one matcher
    whether or not it sees traffic.
  EOT
  type        = list(string)

  validation {
    condition     = length(var.shards) > 0
    error_message = "At least one shard is required."
  }

  validation {
    condition     = length(var.shards) == length(distinct(var.shards))
    error_message = "shards must not contain duplicates; each shard owns exactly one matcher Deployment."
  }
}

variable "shard_precision" {
  description = <<-EOT
    Geohash precision of `shards`, passed to the matcher's PRECISION.

    Must equal `binary_shard_precision`. The orchestrator picks a request
    subject with the precision compiled into it, so a mismatch sends every
    request to a subject no matcher serves.
  EOT
  type        = number

  validation {
    condition     = var.shard_precision >= 1 && var.shard_precision <= 12 && floor(var.shard_precision) == var.shard_precision
    error_message = "shard_precision must be a whole number in 1..12."
  }

  validation {
    condition     = alltrue([for s in var.shards : length(s) == var.shard_precision])
    error_message = "Every shard must be exactly shard_precision characters long, or a matcher loads a different extent from the one its subject covers."
  }
}

variable "binary_shard_precision" {
  description = <<-EOT
    The `event::SHARD_PRECISION` constant compiled into the binaries.

    The orchestrator is geography-blind except for one line: it routes each
    request to `events.match.<shard_of(point)>`, and derives that shard with
    this constant rather than from configuration, because every pod must agree
    on it whether or not it holds chart values. Raising deployed precision is
    therefore a code change, not a variable change — this exists so the plan
    says that instead of the cluster discovering it at runtime.
  EOT
  type        = number
  default     = 4
}

variable "coverage_cells" {
  description = "Optional allowlist of geohash prefixes for the service region. Catches a shard list generated over the wrong extent before it becomes a fleet of idle matchers."
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

variable "hot_shard_replica_factor" {
  description = <<-EOT
    Ceiling multiplier for a shard's matcher HPA, over the replica count the
    mean rate implies.

    Load per shard is not uniform — shards are geographic, traffic is not — and
    the mean is what this model can compute. The HPA absorbs the difference, so
    this is how much skew a hot shard may take up before it queues: 4 means a
    shard carrying four times the mean can still be served.
  EOT
  type        = number
  default     = 4

  validation {
    condition     = var.hot_shard_replica_factor >= 1
    error_message = "hot_shard_replica_factor must be at least 1."
  }
}

# --- Vehicles: the orchestrator half ---------------------------------------

variable "max_matcher_replicas_per_shard" {
  description = <<-EOT
    The point past which one shard's queue group stops being the right
    structure, and the geography should be subdivided instead.

    Every replica of a shard loads that shard's whole `.shard.rt` file, so
    scaling one shard buys throughput at a fixed memory cost per pod, and a
    single Deployment's rollout eventually becomes the slow part. When the mean
    load needs more replicas than this, the model reports a shard deficit and a
    finer precision rather than a taller Deployment.
  EOT
  type        = number
  default     = 32

  validation {
    condition     = var.max_matcher_replicas_per_shard >= 1
    error_message = "max_matcher_replicas_per_shard must be at least 1."
  }
}

variable "partitions" {
  description = <<-EOT
    Vehicle partitions the space divides into: `partition::PARTITIONS`.

    Wire law, not a tunable. Producers compute `splitmix64(vehicle_id) % this`
    to pick a subject, so changing it re-addresses every event in flight. It
    appears here only because the fleet size must divide it.
  EOT
  type        = number
  default     = 1024
}

variable "streams" {
  description = <<-EOT
    Raw JetStream streams the partition space divides across, fleet-wide.

    Wire law with a migration behind it, not a tuning knob: a revision is a
    stream sequence, so moving a partition to another stream resets its
    sequence domain and breaks every revision comparison across the boundary.
    Pin it once, above what the design target needs.

    One stream is one raft leader, which is what makes the count matter at all:
    64 streams put the 5M design target at ~98k writes/s each, inside
    `jetstream_writes_per_stream`. 32 would put it at ~195k, over that ceiling
    — and discovering it later costs a migration rather than an edit.
  EOT
  type        = number
  default     = 64

  validation {
    condition     = var.streams >= 1 && floor(var.streams) == var.streams
    error_message = "streams must be a whole number of at least 1."
  }
}

variable "matched_streams" {
  description = <<-EOT
    Streams the matched emissions divide across.

    Fixed at 1 because `ingest::MATCHED_STREAM` is one stream in the binary.
    Unlike the raw side this carries no sequence law — revisions are minted on
    ingest — so splitting it is a small code change rather than a migration.
    The model reports what the target needs so the gap is visible before an
    apply, not after.
  EOT
  type        = number
  default     = 1
}

variable "vertical_profile" {
  description = "Which `profiles` entry to deploy."
  type        = string
  default     = "standard"
}

variable "profiles" {
  description = <<-EOT
    Vertical scaling steps.

    `matcher_eps` is the sustained request rate one matcher replica serves. It
    is CPU-bound work — a Viterbi solve fanned across rayon — so one number
    describes it.

    The orchestrator is not, and a single number hid that. It is mostly an I/O
    scheduler: a worker holds its vehicle's lane across the whole round trip —
    solve, broker-confirmed publish, batched archive flush — and spends almost
    all of that awaiting rather than computing. So its ceiling is whichever of
    two independent bounds binds first:

      concurrency   `workers / round_trip`. Workers are tokio tasks, so this
                    bound is cheap to raise and has nothing to do with cores.

      compute       the real per-event CPU: decode, context assembly,
                    reconcile, encode. This is what the CPU request buys.

    Holding them apart matters because they are measured differently and fixed
    differently. A pod short on concurrency is one config change away from
    twice the throughput on the same cores; a pod short on CPU is not. Fusing
    them into one `orchestrator_eps` made a concurrency shortfall look like a
    hardware requirement, and bought cores that could not be used.

    Every figure here is an estimate from the shipped defaults rather than a
    measurement. `orchestrator_round_trip_ms` and
    `orchestrator_cpu_micros_per_event` are the two worth measuring first,
    because between them they set the whole pipeline pool.
  EOT
  type = map(object({
    matcher_workers    = number
    matcher_cpu_millis = number
    matcher_memory_mib = number
    matcher_eps        = number

    orchestrator_workers              = number
    orchestrator_round_trip_ms        = number
    orchestrator_cpu_micros_per_event = number
    orchestrator_cpu_millis           = number
    orchestrator_memory_mib           = number
  }))

  default = {
    small = {
      matcher_workers    = 5
      matcher_cpu_millis = 1000
      matcher_memory_mib = 3072
      matcher_eps        = 1500

      orchestrator_workers              = 64
      orchestrator_round_trip_ms        = 8
      orchestrator_cpu_micros_per_event = 50
      orchestrator_cpu_millis           = 500
      orchestrator_memory_mib           = 1024
    }

    # The calibration anchor. Concurrency is sized so neither bound wastes the
    # other: 512 workers at an 8 ms round trip is 64k evt/s of scheduling
    # against 40k evt/s of CPU, so the pod is compute-bound with the
    # concurrency to keep those cores fed through a latency spike.
    #
    # The chart's shipped 64 workers is 8k evt/s — a fifth of what the same
    # cores can do — so a pod sized this way and left at that count would buy
    # CPU it could never reach.
    standard = {
      matcher_workers    = 10
      matcher_cpu_millis = 2000
      matcher_memory_mib = 3072
      matcher_eps        = 6000

      orchestrator_workers              = 512
      orchestrator_round_trip_ms        = 8
      orchestrator_cpu_micros_per_event = 50
      orchestrator_cpu_millis           = 2000
      orchestrator_memory_mib           = 4096
    }

    # Fewer, fatter pods. Fewer objects is its own win: a fleet of thousands
    # of Deployments makes a rollout an API-server problem. It is not a cost
    # win — see the summary's per-event line.
    large = {
      matcher_workers    = 24
      matcher_cpu_millis = 6000
      matcher_memory_mib = 6144
      matcher_eps        = 16000

      orchestrator_workers              = 2048
      orchestrator_round_trip_ms        = 8
      orchestrator_cpu_micros_per_event = 50
      orchestrator_cpu_millis           = 8000
      orchestrator_memory_mib           = 16384
    }
  }
}

variable "max_shards_per_release" {
  description = <<-EOT
    Guard on Helm release size. Every shard renders a Deployment and an HPA
    into one release, whose manifests live in a single Kubernetes Secret, and a
    Secret holds 1 MiB.

    The whole deployment is one release: the orchestrator fleet is a single
    StatefulSet over the vehicle partition space, so it must be rendered
    exactly once, and splitting matchers into their own releases would put the
    two halves of one system behind two apply paths.
  EOT
  type        = number
  default     = 512
}

# --- NATS -------------------------------------------------------------------

variable "nats_min_replicas" {
  description = <<-EOT
    Availability floor for the cluster.

    Three, and now for a reason core NATS did not have: JetStream elects a raft
    leader per stream, so a cluster meant to survive a node loss needs an odd
    membership of at least three. At one replica a lost server takes its
    streams' backlogs with it.
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
    Core deliveries per second one NATS server sustains, for postcard payloads
    of 100B-10KiB. Applies to the request/reply half of the pipeline, which
    does not touch a stream.

    The least trustworthy number in this model. Every official NATS figure is a
    single server over loopback on a laptop — the project publishes no
    clustered benchmarks — and real networked hardware runs well under them.
    This default is the published 4M aggregate for 1 publisher to 4 subscribers
    at 128B, with a 4x haircut.

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

variable "nats_core_hops" {
  description = <<-EOT
    Deliveries per input event that never touch a stream.

    Four: the raw event's delivery from its partition's consumer and the ack
    back, then the solve request and its reply. The two stream *writes* — raw
    ingest and the matched emission — are counted separately, because a
    persisted write and a routed delivery cost different orders of magnitude.
  EOT
  type        = number
  default     = 4
}

variable "jetstream_writes_per_stream" {
  description = <<-EOT
    Persisted messages per second one stream sustains, at file storage and one
    replica.

    Per *stream*, not per server, because a stream has one raft leader and that
    leader is where its writes serialise. This is the number the stream count
    exists to divide, and the reason the count cannot be small.

    Unmeasured. Published JetStream figures put a single stream between 2.5x
    and 24x under core NATS on the same hardware; this default sits an order of
    magnitude under core, which is the conservative end of that range for
    cloud-attached SSD. Measure with `nats bench --js` on the real node shape
    and disk class before trusting it, because the stream count it justifies
    cannot be changed afterwards without a migration.
  EOT
  type        = number
  default     = 150000

  validation {
    condition     = var.jetstream_writes_per_stream > 0
    error_message = "jetstream_writes_per_stream must be positive."
  }
}

variable "jetstream_writes_per_server" {
  description = "Persisted messages per second one server sustains across every stream it leads. Bounds how many stream leaders a server can hold, which is what decides cluster size once the per-stream ceiling is satisfied."
  type        = number
  default     = 400000

  validation {
    condition     = var.jetstream_writes_per_server > 0
    error_message = "jetstream_writes_per_server must be positive."
  }
}

variable "jetstream_stream_replicas" {
  description = <<-EOT
    Replicas per stream, and so the write amplification into the cluster.

    One, because `ingest.rs` does not set `num_replicas` and JetStream
    defaults to one. That is a deliberate gap to know about rather than a
    setting: at R1 a lost server loses its streams' unprocessed backlog, since
    a work queue holds the only copy of an event until it is acked. Raising it
    is a change to the stream config in `ingest.rs`, not to this variable.
  EOT
  type        = number
  default     = 1
}

variable "jetstream_messages_per_write" {
  description = <<-EOT
    Messages a stream commits per disk write.

    JetStream appends and fsyncs in batches rather than once per message, so
    the volume's operation count is well under the message rate. The softest
    number in the disk model, and it only sets IOPS — throughput is derived
    from bytes and is usually what binds first, since the appends are
    sequential.
  EOT
  type        = number
  default     = 16

  validation {
    condition     = var.jetstream_messages_per_write >= 1
    error_message = "jetstream_messages_per_write must be at least 1."
  }
}

variable "raw_event_bytes" {
  description = "Wire size of one postcard-encoded raw event, including its JetStream framing and subject. Sizes the work-queue backlog on disk."
  type        = number
  default     = 256
}

variable "matched_event_bytes" {
  description = <<-EOT
    Wire size of one matched emission.

    Far above a raw event, and deliberately: a matcher emits the entire cut
    trip on every solve, so an emission carries every layer since the
    convergence point rather than a delta. That is what makes competing solves
    resolvable by revision, and it is also why the matched stream's byte rate
    is a multiple of the raw one.
  EOT
  type        = number
  default     = 2048
}

variable "raw_backlog_seconds" {
  description = <<-EOT
    Seconds of raw events the work queues must hold on disk.

    A work queue deletes a message on ack, so steady-state disk is near zero
    and this sizes the abnormal case: how long the orchestrator fleet may be
    down, or wedged behind a sick matcher shard, before ingest has nowhere to
    put events.
  EOT
  type        = number
  default     = 900
}

variable "matched_retention_seconds" {
  description = "How long the matched stream retains emissions. Nothing consumes it destructively, so this is the reconciler's worst tolerable lag, and it sizes that stream's disk outright."
  type        = number
  default     = 900
}

variable "nats_cpu_millis" {
  description = "CPU request per NATS server. Core routing is a network loop, but JetStream adds raft and file I/O on top, so this buys both."
  type        = number
  default     = 4000
}

variable "nats_memory_mib" {
  description = "Memory request per NATS server: connection buffers, slow-consumer queues, and JetStream's per-stream and per-consumer state. One durable consumer per partition means this scales with the partition count, not just with traffic."
  type        = number
  default     = 8192
}

# --- Valkey -----------------------------------------------------------------

variable "valkey_ops_per_primary" {
  description = <<-EOT
    Commands per second one Valkey primary sustains.

    The figure measured in Valkey's 1-billion-RPS cluster run: 2000 primaries
    on r7g.2xlarge (8 vCPU) with 6 io-threads, ~500k ops/s each, growing almost
    linearly with primary count. https://valkey.io/blog/1-billion-rps/

    Single-node benchmarks reach ~1.19M ops/s, but with no replication and no
    cluster bus, so that number does not describe a fleet member.
  EOT
  type        = number
  default     = 500000

  validation {
    condition     = var.valkey_ops_per_primary > 0
    error_message = "valkey_ops_per_primary must be positive."
  }
}

variable "valkey_ops_per_event" {
  description = <<-EOT
    Commands per input event: one `XADD ... MAXLEN ~` appending the vehicle's
    raw tail, trimmed in the same command.

    One, where the previous topology needed two. The orchestrator absorbed the
    historian's batched write, and its per-event read is gone: a vehicle's
    history lane is warmed once per ownership and held in the pod, so the
    `XREVRANGE` amortises to near zero over the vehicle's stay. Raise it only
    if lane eviction becomes common, which means `vehicle_cache` is too small.
  EOT
  type        = number
  default     = 1
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

variable "valkey_replicas_per_primary" {
  description = "Read replicas per primary, for failover only. The pipeline never reads a replica: a warming lane must see the tail its own pod wrote."
  type        = number
  default     = 1
}

variable "valkey_cpu_millis" {
  description = "CPU request per Valkey pod. No CPU limit is set: a CFS cap would freeze the command loop in 100ms windows and queue every in-flight command behind it."
  type        = number
  default     = 2000
}

variable "valkey_memory_mib" {
  description = "Memory request per Valkey pod. One stream per vehicle trimmed to HISTORY entries, so bounded by concurrent vehicles rather than by event rate."
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

# --- Telemetry --------------------------------------------------------------

variable "spans_per_event" {
  description = "Spans emitted per input event across both services: the queue waits, the solve round trip, the publish, the archive flush and the commit."
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

# --- Cost -------------------------------------------------------------------

variable "pricing_commitment" {
  description = <<-EOT
    Which rate to cost the compute at.

    Commitments apply to the nodes only; disk and the cluster fee are charged
    the same either way. Spot is priced here for comparison and is not a
    configuration this deployment should adopt wholesale — an orchestrator's
    partitions have no other owner while it reschedules.
  EOT
  type        = string
  default     = "on_demand"

  validation {
    condition     = contains(["on_demand", "cud_1y", "cud_3y", "spot"], var.pricing_commitment)
    error_message = "pricing_commitment must be on_demand, cud_1y, cud_3y or spot."
  }
}

variable "boot_disk_gib" {
  description = "Boot disk per node, matching the platform module's `node_disk_size_gb`. Image layers under streaming, so it holds little and is priced at the class baseline."
  type        = number
  default     = 100
}

variable "provisioned_iops" {
  description = "IOPS provisioned per file store volume, matching the devstack. Only what exceeds the class baseline is billable."
  type        = number
  default     = 30000
}

variable "provisioned_throughput_mib" {
  description = "MiB/s provisioned per file store volume, matching the devstack. Only what exceeds the class baseline is billable."
  type        = number
  default     = 750
}

variable "prices" {
  description = <<-EOT
    Monthly rates, in USD.

    Read for australia-southeast1 (Sydney) on 2026-08-05. Sydney is well above
    the US regions — roughly 1.35x — so these do not transfer to another
    region by inspection.

    The machine rates are per node per month at 730 hours, taken from the
    published per-region tables. The Hyperdisk figures are the weaker set: the
    per-GiB Sydney rate was not published in a form that could be read
    directly and is extrapolated from the Sydney/US ratio on pd-balanced,
    while the IOPS and throughput rates are the published US figures and are
    probably understated for Sydney. Treat the storage lines as an estimate
    and the compute lines as quotes.

    Confirm against the billing account before anything depends on the total.
  EOT

  type = object({
    machines = map(object({
      on_demand = number
      cud_1y    = number
      cud_3y    = number
      spot      = number
    }))

    hyperdisk_balanced_gib = number
    hyperdisk_iops         = number
    hyperdisk_mib          = number
    hyperdisk_free_iops    = number
    hyperdisk_free_mib     = number

    gke_cluster_hour = number
  })

  default = {
    # Per node per month at 730 hours. Spot is the hourly rate scaled the same
    # way, for comparison rather than for use.
    machines = {
      "c4-highcpu-32"  = { on_demand = 1241.76, cud_1y = 782.33, cud_3y = 558.80, spot = 548.52 }
      "c4-highcpu-16"  = { on_demand = 620.88, cud_1y = 391.16, cud_3y = 279.40, spot = 274.26 }
      "c4-standard-16" = { on_demand = 721.50, cud_1y = 454.55, cud_3y = 324.67, spot = 318.72 }
      "c4-standard-8"  = { on_demand = 360.75, cud_1y = 227.28, cud_3y = 162.34, spot = 159.36 }
    }

    # Estimated from the Sydney/US ratio on pd-balanced, which is published at
    # $0.135/GB there against $0.10 in the US.
    hyperdisk_balanced_gib = 0.147

    # Published US rates for provisioning above the baseline. Sydney is
    # probably higher, so these understate the storage line.
    hyperdisk_iops = 0.005
    hyperdisk_mib  = 0.040

    # The baseline every Hyperdisk Balanced volume includes at no charge.
    hyperdisk_free_iops = 3000
    hyperdisk_free_mib  = 140

    # One regional cluster. A billing account gets one zonal cluster free,
    # which a regional cluster is not.
    gke_cluster_hour = 0.10
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
