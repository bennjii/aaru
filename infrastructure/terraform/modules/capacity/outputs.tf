output "shard_count" {
  description = "Shards deployed; the number of matcher Deployments."
  value       = local.shard_count
}

output "shard_precision" {
  description = "Geohash precision, passed through to the matcher's PRECISION."
  value       = var.shard_precision
}

output "profile" {
  description = "Resolved vertical profile: per-pod resources, worker counts and per-pod throughput."
  value       = local.profile
}

output "required_eps" {
  description = "Target plus headroom; what every capacity figure must reach."
  value       = local.required_eps
}

output "meets_target" {
  description = "Whether matchers, the fleet, the stream topology and Valkey all reach the target. Env roots assert on this."
  value       = local.meets_target
}

# --- Matchers ---------------------------------------------------------------

output "matcher" {
  description = <<-EOT
    The geographic half. Replicas divide a shard's load rather than duplicating
    it, because matchers serve their shard subject through a NATS queue group,
    so `replicas` is a real horizontal lever and not just redundancy.

    `replicas` is the floor derived from the mean rate; `replicas_max` is the
    HPA ceiling, which is what absorbs the difference between a mean and a
    geographic distribution.

    `pods_floor` is the count throughput alone demands. Shards do not multiply
    the pod count — they divide it, and the two cancel — so the gap between
    `pods` and `pods_floor` is pure per-shard rounding, and finer geography
    widens it. Precision is worth spending on shard-file size and on how
    finely load can be isolated, not on pod count, and it is worth spending
    carefully: a finer grid means more trips crossing a shard boundary, and
    every crossing degrades a resume to a restart.
  EOT
  value = {
    shards         = local.shard_count
    replicas       = local.matcher_replicas_per_shard
    replicas_max   = local.matcher_replicas_max
    pods           = local.matcher_pods
    pods_max       = local.shard_count * local.matcher_replicas_max
    pods_floor     = local.matcher_pods_floor
    eps_per_pod    = local.profile.matcher_eps
    capacity_eps   = local.matcher_capacity_eps
    mean_shard_eps = local.mean_shard_eps

    # 1.0 means the geography costs nothing over the throughput floor.
    rounding_overhead = local.matcher_rounding_overhead

    # Memory is set by geography, not by the profile: the graph a shard makes
    # a pod hold. `graph_mib_total` is the fleet-wide duplication — every
    # replica loads its shard's whole graph, and the pod count is fixed by
    # throughput, so a bigger shard cannot be offset by having fewer of them.
    memory_mib      = local.matcher_memory_mib
    graph_mib       = local.matcher_graph_mib
    graph_mib_total = local.matcher_graph_mib_total
  }
}

output "shard_memory_note" {
  description = <<-EOT
    How a shard's size lands on the fleet, and which resource ends up binding
    the matcher pool.

    Worth reading before changing precision. Pod count is throughput's to
    decide, so coarsening the grid does not buy fewer pods — it makes each one
    hold a bigger graph, and the pool tips from CPU-bound to memory-bound.
  EOT
  value = join(" ", [
    "The largest shard is ${var.largest_shard_file_mib} MiB on disk and",
    "${local.matcher_graph_mib} MiB resident at ${var.shard_memory_expansion}x,",
    "so every matcher is sized at ${local.matcher_memory_mib} MiB including its working set.",
    "Across ${local.matcher_pods} pods that is ${format("%.1f", local.matcher_graph_mib_total / 1024)} GiB of graph,",
    "most of it the same shards loaded again per replica.",
    "One node fits ${floor(local.allocatable["matcher"].memory_mib / local.matcher_memory_mib)} by memory",
    "and ${floor(local.allocatable["matcher"].cpu_millis / local.profile.matcher_cpu_millis)} by CPU,",
    "so the pool is ${
      floor(local.allocatable["matcher"].memory_mib / local.matcher_memory_mib) <
      floor(local.allocatable["matcher"].cpu_millis / local.profile.matcher_cpu_millis)
      ? "memory-bound — a finer grid would shrink the graph and recover nodes"
      : "CPU-bound, so the graph is currently free"
    }.",
  ])
}

output "shards_required" {
  description = "Shards needed at this profile for one shard's load to stay inside its HPA ceiling. Below the current count, replicas are the lever instead."
  value       = local.shards_required
}

output "shard_deficit" {
  description = "Shards missing before a single shard's mean load exceeds what its replicas can serve; zero when replicas suffice."
  value       = local.shard_deficit
}

output "recommended_precision" {
  description = "Precision that would close the shard deficit. Regenerate the shard cache at this precision before adopting it."
  value       = local.recommended_precision
}

# --- Orchestrator fleet -----------------------------------------------------

output "fleet" {
  description = <<-EOT
    The vehicle-partitioned half: a StatefulSet whose pods take disjoint
    ordinal slices of the partition space.

    `size` is snapped to a divisor of `partitions` so every slice is equal —
    the binary gives the last pod the remainder, which would otherwise make it
    the hot one. Resizing re-slices the whole space, so it is a quiesced
    operation, or a window of double ownership that revisions make wasteful
    rather than wrong.
  EOT
  value = {
    size               = local.fleet
    required           = local.fleet_required
    partitions         = var.partitions
    partitions_per_pod = local.partitions_per_pod
    eps_per_pod        = local.orchestrator_eps
    capacity_eps       = local.orchestrator_capacity_eps
    fits_partitions    = local.fleet_fits_the_partition_space

    # Which ceiling the pod actually hits, and what the other one would allow.
    # `concurrency` means the fix is a worker count, not a bigger pod.
    bound               = local.orchestrator_bound
    io_eps              = local.orchestrator_io_eps
    cpu_eps             = local.orchestrator_cpu_eps
    workers             = local.profile.orchestrator_workers
    round_trip_ms       = local.profile.orchestrator_round_trip_ms
    stranded_cpu_millis = local.orchestrator_stranded_cpu_millis
  }
}

output "orchestrator_efficiency_note" {
  description = "Non-empty when the fleet is bought CPU it cannot reach, because concurrency rather than compute is what binds each pod."
  value = local.orchestrator_bound != "concurrency" ? "" : join(" ", [
    "Each orchestrator is concurrency-bound at ${local.orchestrator_io_eps} evt/s,",
    "against the ${local.orchestrator_cpu_eps} evt/s its CPU request could serve.",
    "That strands ${format("%.0f", local.orchestrator_stranded_cpu_millis)}m per pod across a fleet of ${local.fleet}.",
    "Workers are tokio tasks holding a lane across a round trip, not threads,",
    "so raising orchestrator_workers costs memory rather than cores —",
    "and VEHICLE_CACHE is per worker, so lower it in step or the pod's",
    "lane memory grows with the same change.",
  ])
}

# --- Streams ----------------------------------------------------------------

output "streams" {
  description = <<-EOT
    The JetStream ingest topology, and the one part of this model that cannot
    be corrected later: a revision is a stream sequence, so moving a partition
    between streams resets its sequence domain.

    `writes_per_stream` against `jetstream_writes_per_stream` is the binding
    constraint, because a stream has a single raft leader.
  EOT
  value = {
    raw                   = var.streams
    matched               = var.matched_streams
    partitions_per_stream = local.partitions_per_stream
    consumers_per_stream  = local.partitions_per_stream
    stream_replicas       = var.jetstream_stream_replicas

    raw_writes_per_stream = local.raw_writes_per_stream
    raw_streams_required  = local.raw_streams_required
    raw_sufficient        = local.raw_streams_ok

    matched_writes_per_stream = local.matched_writes_per_stream
    matched_streams_required  = local.matched_streams_required
    matched_sufficient        = local.matched_streams_ok

    disk_bytes_total      = local.jetstream_disk_bytes
    file_store_gib_server = local.file_store_gib_server
  }
}

output "matched_stream_prerequisite" {
  description = <<-EOT
    Non-empty when the matched stream cannot absorb the emission rate.

    Every partition publishes into `ingest::MATCHED_STREAM`, which is one
    stream and so one raft leader. Splitting it is a small change — it carries
    no sequence law, because revisions are minted on the raw side — but it is a
    code change, not a variable.
  EOT
  value = local.matched_streams_ok ? "" : join(" ", [
    "The matched stream must absorb ${format("%d", floor(local.required_eps))} emissions/s,",
    "but one stream sustains ${var.jetstream_writes_per_stream}.",
    "That needs ${local.matched_streams_required} streams;",
    "`ingest.rs` publishes every partition into a single MATCHED_STREAM.",
    "Partition it the way the raw side already is — it carries no sequence law,",
    "since revisions come from the ingest streams — then raise matched_streams.",
  ])
}

output "raw_stream_prerequisite" {
  description = "Non-empty when the pinned raw stream count cannot absorb ingest. Unlike the matched stream this cannot be fixed after events exist."
  value = local.raw_streams_ok ? "" : join(" ", [
    "Raw ingest is ${format("%d", floor(local.raw_writes_per_stream))} writes/s per stream",
    "across ${var.streams} streams, over the ${var.jetstream_writes_per_stream} one leader sustains.",
    "Raise streams to ${local.raw_streams_required} BEFORE any events exist:",
    "revisions are stream sequences, so remapping partitions later resets",
    "sequence domains and breaks every revision comparison across the boundary.",
  ])
}

# --- Dependencies -----------------------------------------------------------

output "nats" {
  description = <<-EOT
    The single NATS cluster, now carrying JetStream as well as core routing.
    Sized from whichever binds first: routed deliveries, or persisted writes.

    Replica count is odd because JetStream elects a leader per stream, and an
    even membership adds a vote to every quorum without buying fault tolerance
    over the odd number below it.
  EOT
  value = {
    replicas_total = local.nats_replicas_total

    core_delivery_rate = local.core_delivery_rate
    msgs_per_server    = var.nats_msgs_per_server

    jetstream_write_rate = local.jetstream_write_rate
    writes_per_server    = var.jetstream_writes_per_server
    stream_leaders       = local.stream_leaders_total

    file_store_gib = local.file_store_gib_server
    cpu_millis     = var.nats_cpu_millis
    memory_mib     = var.nats_memory_mib

    # How many nodes the infra pool must have for the servers to spread. A
    # node may hold at most half the cluster short of one, or losing it costs
    # quorum on every stream that node was leading.
    spread_nodes = local.nats_spread_nodes
  }
}

output "jetstream_disk" {
  description = <<-EOT
    What the file store must sustain per server, as opposed to how much it must
    hold. Capacity and performance are separate constraints here, and a pd-*
    class conflates them: its throughput scales with size, so the broker's
    write ceiling would depend on the retention window rather than on traffic.
    The devstack provisions both explicitly from these figures.

    Throughput usually binds before IOPS — JetStream appends sequentially — and
    the matched stream is most of the bytes, because an emission carries the
    whole cut trip rather than a delta.
  EOT
  value = {
    write_mib_per_server = local.jetstream_write_mib_server
    iops_per_server      = local.jetstream_iops_server
    write_bytes_rate     = local.jetstream_write_bytes_rate
    gib_per_server       = local.file_store_gib_server
    messages_per_write   = var.jetstream_messages_per_write
  }
}

output "valkey" {
  description = <<-EOT
    The Valkey fleet. One logical keyspace, sharded by `hash(vehicle_id)`
    rather than by geography, so no boundary affects history continuity.

    Halved against the previous topology: the orchestrator absorbed the
    historian's batched write and its per-event read is gone, so an event costs
    one `XADD` where it used to cost a read and a write.
  EOT
  value = {
    client_mode          = var.valkey_client_mode
    primaries            = local.valkey_primaries_total
    pods                 = local.valkey_pods_total
    replicas_per_primary = var.valkey_replicas_per_primary
    ops_rate             = local.valkey_ops_rate
    ops_per_event        = var.valkey_ops_per_event
    ops_per_primary      = var.valkey_ops_per_primary
    io_threads           = var.valkey_io_threads
    cpu_millis           = var.valkey_cpu_millis
    memory_mib           = var.valkey_memory_mib
  }
}

output "valkey_client_prerequisite" {
  description = "Non-empty when the fleet is larger than the client can address. Adding primaries does not help: the client must be able to reach them."
  value = local.valkey_client_is_sufficient ? "" : join(" ", [
    "valkey_client_mode is 'single' but the target needs ${local.valkey_primaries_total} primaries.",
    "Clients would reach one primary and the deployment would cap at",
    "${local.single_valkey_ceiling_eps} evt/s.",
    "Switch to 'pooled-hash': RedisStore already spreads vehicles across the",
    "whole fleet by rendezvous hash.",
  ])
}

output "single_valkey_ceiling_eps" {
  description = "Event rate one primary serves, and the whole deployment's ceiling under `valkey_client_mode = \"single\"`."
  value       = local.single_valkey_ceiling_eps
}

output "telemetry" {
  description = "Collector sizing and the sample ratio the services must apply. Rendered into the workloads as the standard OTEL_TRACES_SAMPLER env pair."
  value = {
    sample_ratio       = var.telemetry_sample_ratio
    span_rate          = local.span_rate
    collector_replicas = local.collector_replicas
    cpu_millis         = var.collector_cpu_millis
    memory_mib         = var.collector_memory_mib
  }
}

output "out_of_coverage_shards" {
  description = "Shards not nesting under any `coverage_cells` entry. Must be empty when coverage is declared."
  value       = local.out_of_coverage
}

output "precision_prerequisite" {
  description = "Non-empty when the deployed shard precision disagrees with the one compiled into the binaries, which would send every request to a subject no matcher serves."
  value = var.shard_precision == var.binary_shard_precision ? "" : join(" ", [
    "shard_precision is ${var.shard_precision} but the binaries route with",
    "event::SHARD_PRECISION = ${var.binary_shard_precision}.",
    "The orchestrator derives a request subject from the compiled constant, so",
    "every solve would address `events.match.<${var.binary_shard_precision}-char>`",
    "while matchers serve `<${var.shard_precision}-char>` subjects and nothing replies.",
    "Change the constant and rebuild, or deploy shards at precision ${var.binary_shard_precision}.",
  ])
}

# --- The design target ------------------------------------------------------

output "design_target" {
  description = <<-EOT
    What the design target would need, for comparison against what is pinned
    now. `streams_ok` and `fleet_fits` are the two that matter: both quantities
    are wire law, so they are cheap today and a migration later.
  EOT
  value       = local.design
}

# --- Nodes ------------------------------------------------------------------

output "pools" {
  description = "Node pool sizing per pool: machine type, autoscaler floor and ceiling, allocatable capacity, and the demand behind it."
  value       = local.pools
}

output "unschedulable_shapes" {
  description = "Pod shapes larger than one node's allocatable capacity, as \"<pool>/<shape>\". Must be empty."
  value       = local.unschedulable_shapes
}

output "totals" {
  description = "Fleet totals, for cost estimation and for sizing the pod CIDR."
  value = {
    nodes            = local.total_nodes
    vcpu             = local.total_vcpu
    pods             = local.total_pods
    pod_ips_required = local.pod_ips_required
  }
}

output "cost" {
  description = <<-EOT
    Estimated monthly spend in USD, at the autoscaler floor.

    `by_pool` is what is actually bought — nodes come whole. `by_service`
    attributes those nodes by CPU share, which is exact for the three pools
    carrying one service each and a split only for `infra`, between the
    brokers and the keyspace.

    `compute_max` is the matcher HPA's ceiling rather than a forecast: it is
    what a sustained fleet-wide burst would cost, and it is paid only while
    the burst lasts.
  EOT
  value = {
    currency   = "USD"
    region     = "australia-southeast1"
    commitment = var.pricing_commitment

    by_pool    = local.pool_monthly
    by_service = local.service_monthly

    compute     = local.compute_monthly
    compute_max = local.compute_monthly_max

    storage = {
      boot_disks            = local.boot_disk_monthly
      jetstream_capacity    = local.jetstream_capacity_monthly
      jetstream_performance = local.jetstream_performance_monthly
      total                 = local.storage_monthly
    }

    cluster_fee = local.cluster_monthly

    total     = local.total_monthly
    total_max = local.total_monthly_max

    per_million_events = local.cost_per_million_events
  }
}

output "cost_report" {
  description = "The cost breakdown as a table. `tofu output -raw cost_report` in an env root."
  value = <<-EOT
    ${var.pricing_commitment} rates, australia-southeast1, USD/month at ${local.hours_per_month}h

    by node pool
    ${join("\n    ", [
  for p, v in local.pool_monthly :
  format("%-10s %3d x %-16s %10s", p, local.node_counts[p], var.machines[p].machine_type, format("$%.0f", v))
  ])}
    ${format("%-32s %10s", "compute subtotal", format("$%.0f", local.compute_monthly))}

    by service (nodes attributed by cpu share)
    ${join("\n    ", [
  for s, v in local.service_monthly :
  format("%-32s %10s", s, format("$%.0f", v + local.service_storage_monthly[s]))
])}

    storage
    ${format("%-32s %10s", "boot disks (${local.total_nodes} x ${var.boot_disk_gib}GiB)", format("$%.0f", local.boot_disk_monthly))}
    ${format("%-32s %10s", "jetstream capacity (${local.nats_replicas_total} x ${local.file_store_gib_server}GiB)", format("$%.0f", local.jetstream_capacity_monthly))}
    ${format("%-32s %10s", "jetstream iops + throughput", format("$%.0f", local.jetstream_performance_monthly))}

    ${format("%-32s %10s", "gke cluster fee", format("$%.0f", local.cluster_monthly))}
    ${format("%-32s %10s", "TOTAL", format("$%.0f", local.total_monthly))}
    ${format("%-32s %10s", "at the matcher HPA ceiling", format("$%.0f", local.total_monthly_max))}
    ${format("%-32s %10s", "per million events", format("$%.4f", local.cost_per_million_events))}
  EOT
}

output "summary" {
  description = "Human-readable sizing report. `tofu output -raw capacity_summary` in an env root."
  value       = <<-EOT
    target        ${format("%d", var.throughput_target_eps)} evt/s (+${format("%d", floor(var.headroom_ratio * 100))}% headroom = ${format("%d", floor(local.required_eps))})
    profile       ${var.vertical_profile} (matcher ${local.profile.matcher_eps} evt/s x ${local.profile.matcher_workers}w, orchestrator ${local.orchestrator_eps} evt/s x ${local.profile.orchestrator_workers}w, ${local.orchestrator_bound}-bound)
    verdict       ${local.meets_target ? "MEETS TARGET" : "SHORT"}

    matchers      ${local.shard_count} shards x ${local.matcher_replicas_per_shard} replicas = ${local.matcher_pods} pods (HPA to ${local.matcher_replicas_max}/shard, ${local.shard_count * local.matcher_replicas_max} pods)
                  ${format("%d", floor(local.mean_shard_eps))} evt/s mean per shard, capacity ${format("%d", floor(local.matcher_capacity_eps))} evt/s
                  throughput floor is ${local.matcher_pods_floor} pods; this geography costs ${format("%.2f", local.matcher_rounding_overhead)}x that
    fleet         ${local.fleet} pods x ${local.partitions_per_pod} partitions = ${var.partitions}, capacity ${format("%d", floor(local.orchestrator_capacity_eps))} evt/s
    streams       ${var.streams} raw (${format("%d", floor(local.raw_writes_per_stream))} writes/s each, ceiling ${var.jetstream_writes_per_stream}), ${local.partitions_per_stream} consumers each
                  ${var.matched_streams} matched (${format("%d", floor(local.matched_writes_per_stream))} writes/s each)
    nats          ${local.nats_replicas_total} servers over >=${local.nats_spread_nodes} nodes, ${local.stream_leaders_total} stream leaders
    file store    ${local.file_store_gib_server} GiB per server at ${local.jetstream_write_mib_server} MiB/s, ${local.jetstream_iops_server} IOPS
    valkey        ${local.valkey_primaries_total} primaries + ${local.valkey_primaries_total * var.valkey_replicas_per_primary} replicas, ${var.valkey_client_mode}, ${var.valkey_io_threads} io-threads
    collectors    ${local.collector_replicas} at ${var.telemetry_sample_ratio} sample ratio (${format("%d", floor(local.span_rate))} spans/s)

    ${local.raw_streams_ok ? "streams       raw sufficient" : format("STREAMS       raw SHORT: need %d, have %d -- pin before events exist", local.raw_streams_required, var.streams)}
    ${local.matched_streams_ok ? "streams       matched sufficient" : format("STREAMS       matched SHORT: need %d, have %d -- partition MATCHED_STREAM in ingest.rs", local.matched_streams_required, var.matched_streams)}
    ${local.fleet_fits_the_partition_space ? "fleet         fits the partition space" : format("FLEET         SHORT: needs %d pods for %d partitions -- use a larger profile", local.fleet_required, var.partitions)}
    ${local.shard_deficit > 0 ? format("shards        short by %d; try precision %d", local.shard_deficit, local.recommended_precision) : "shards        sufficient"}
    ${local.valkey_client_is_sufficient ? "valkey        sufficient" : format("VALKEY        client mode 'single' caps this at %d evt/s; needs pooled-hash for %d primaries", local.single_valkey_ceiling_eps, local.valkey_primaries_total)}
    ${var.shard_precision == var.binary_shard_precision ? "precision     agrees with the binaries" : format("PRECISION     %d disagrees with the compiled %d -- requests would reach no matcher", var.shard_precision, var.binary_shard_precision)}

    design target ${format("%d", var.design_target_eps)} evt/s would need:
                  ${local.design.matcher_pods} matcher pods, a fleet of ${local.design.fleet_required}, ${local.design.valkey_primaries} valkey primaries
                  ${local.design.streams_required} raw streams (${format("%d", floor(local.design.writes_per_stream))} writes/s each at the pinned ${var.streams}) ${local.design.streams_ok ? "-- pinned count survives" : "-- PINNED COUNT IS SHORT"}
                  ${local.design.matched_streams_required} matched streams

    nodes         ${local.total_nodes} total, ${local.total_vcpu} vCPU, ${local.pod_ips_required} pod IPs
    ${join("\n    ", [for p, v in local.pools : format("%-14s%2d-%2d x %-20s %4d pods, cpu %d%%", p, v.min_node_count, v.max_node_count, v.machine_type, v.demand.pods, floor(v.cpu_utilisation * 100))])}
  EOT
}
