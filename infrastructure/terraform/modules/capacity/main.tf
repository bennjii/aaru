# The sizing model. Pure arithmetic, no resources, so it plans without
# credentials and `tofu test` exercises it directly.
#
# The pipeline scales along two independent axes, because its two services
# partition on different things:
#
#   matchers       geographic. One Deployment per shard, replicas behind a
#                  NATS queue group, so replica count divides a shard's load
#                  rather than duplicating it. Levers: shard_precision
#                  (horizontal) and replicas per shard (also horizontal).
#
#   orchestrators  vehicle-partitioned. One StatefulSet whose pods take
#                  disjoint ordinal slices of `partitions`, which is wire law.
#                  Lever: fleet size, bounded by the partition count.
#
# Neither axis constrains the other: a matcher is a pure request/reply solver
# that holds nothing between requests, and an orchestrator owns vehicles
# wherever they drive. What couples them is one line of routing —
# `events.match.<shard_of(point)>` — and `binary_shard_precision` is how this
# model checks the two agree on it.
#
# Two quantities are wire law and cannot be tuned after events exist:
# `partitions`, because producers hash into it, and `streams`, because a
# revision is a stream sequence. Both are checked against the design target
# rather than the current one, since exceeding them later costs a migration.

locals {
  profile      = var.profiles[var.vertical_profile]
  shard_count  = length(var.shards)
  required_eps = var.throughput_target_eps * (1 + var.headroom_ratio)

  # Shards outside the declared service region. A shard list generated over
  # the wrong extent otherwise becomes a fleet of idle matchers.
  out_of_coverage = length(var.coverage_cells) == 0 ? [] : [
    for s in var.shards : s
    if !anytrue([for c in var.coverage_cells : startswith(s, c)])
  ]

  # --- Matchers -----------------------------------------------------------

  # The mean is all this model can compute; `hot_shard_replica_factor` is what
  # covers the difference between it and a geographic distribution.
  mean_shard_eps = local.required_eps / local.shard_count

  matcher_replicas_per_shard = max(1, ceil(local.mean_shard_eps / local.profile.matcher_eps))
  matcher_replicas_max       = ceil(local.matcher_replicas_per_shard * var.hot_shard_replica_factor)

  matcher_pods         = local.shard_count * local.matcher_replicas_per_shard
  matcher_capacity_eps = local.matcher_pods * local.profile.matcher_eps

  # The fewest matcher pods the target could ever need, ignoring geography
  # entirely. Worth stating because the intuition it corrects is a strong one:
  # shards do not multiply the pod count, they divide it, and the two cancel —
  # `shard_count * (required / shard_count / matcher_eps)` is just
  # `required / matcher_eps`.
  #
  # What does not cancel is the rounding. Each shard rounds its replicas up to
  # a whole pod, so the fleet pays that remainder once per shard, and finer
  # geography costs pods rather than saving them. Past `pods_floor` shards
  # every shard is at its one-replica minimum and the pod count simply follows
  # the shard count.
  matcher_pods_floor = max(1, ceil(local.required_eps / local.profile.matcher_eps))

  matcher_rounding_overhead = local.matcher_pods / local.matcher_pods_floor

  # Replicas are the first lever and shards the second, so a shard deficit is
  # not "too few shards to carry the load" — replicas carry it — but "one
  # shard now needs more replicas than a single queue group should hold".
  shards_required = ceil(
    local.required_eps / (local.profile.matcher_eps * var.max_matcher_replicas_per_shard)
  )
  shard_deficit = max(0, local.shards_required - local.shard_count)

  precision_levels_needed = local.shard_deficit == 0 ? 0 : ceil(
    log(local.shards_required / local.shard_count, var.shard_fanout_per_precision)
  )
  recommended_precision = min(12, var.shard_precision + local.precision_levels_needed)

  # --- Orchestrator fleet -------------------------------------------------

  # Pods take contiguous ordinal blocks of the partition space, the last one
  # absorbing the remainder, so a fleet that does not divide `partitions`
  # leaves one pod carrying more than the rest. Snapping to a divisor keeps
  # every slice equal — and every divisor of a power-of-two partition count is
  # itself a power of two.
  fleet_divisors = [
    for d in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024] :
    d if d <= var.partitions && var.partitions % d == 0
  ]

  fleet_required = max(1, ceil(local.required_eps / local.profile.orchestrator_eps))
  fleet_eligible = [for d in local.fleet_divisors : d if d >= local.fleet_required]

  # No eligible divisor means the target needs more pods than there are
  # partitions to divide. Pinned at the ceiling so the rest of the model stays
  # arithmetically sound; `fleet_fits_the_partition_space` reports it.
  fleet = length(local.fleet_eligible) > 0 ? local.fleet_eligible[0] : var.partitions

  partitions_per_pod        = var.partitions / local.fleet
  orchestrator_capacity_eps = local.fleet * local.profile.orchestrator_eps

  # One pod per partition is the hard ceiling: a partition is the smallest
  # thing an owner can hold, so past this the answer is a fatter profile.
  fleet_fits_the_partition_space = local.fleet_required <= var.partitions

  # --- JetStream ----------------------------------------------------------

  # Per stream, because a stream has one raft leader and that leader is where
  # its writes serialise. This is the constraint the stream count exists to
  # divide, and the one that cannot be relieved later without a migration.
  raw_writes_per_stream = local.required_eps / var.streams
  raw_streams_required  = ceil(local.required_eps / var.jetstream_writes_per_stream)
  raw_streams_ok        = local.raw_writes_per_stream <= var.jetstream_writes_per_stream

  # Emissions carry the whole cut trip, so this stream's byte rate is a
  # multiple of the raw one even though its message rate matches.
  matched_writes_per_stream = local.required_eps / var.matched_streams
  matched_streams_required  = ceil(local.required_eps / var.jetstream_writes_per_stream)
  matched_streams_ok        = local.matched_writes_per_stream <= var.jetstream_writes_per_stream

  # One durable pull consumer per partition, each filtering one subject. The
  # count per stream is worth watching independently of throughput: consumer
  # state is raft state, and it is held by the stream's leader.
  partitions_per_stream = ceil(var.partitions / var.streams)

  # A work queue deletes on ack, so this is the abnormal case: how much disk
  # an outage's worth of unacked events needs.
  raw_disk_bytes = (
    local.required_eps * var.raw_event_bytes
    * var.raw_backlog_seconds * var.jetstream_stream_replicas
  )

  matched_disk_bytes = (
    local.required_eps * var.matched_event_bytes
    * var.matched_retention_seconds * var.jetstream_stream_replicas
  )

  jetstream_disk_bytes = local.raw_disk_bytes + local.matched_disk_bytes

  # --- NATS ---------------------------------------------------------------

  # Deliveries that never touch a stream: the solve request and its reply.
  core_delivery_rate = local.required_eps * var.nats_core_hops

  # Raw ingest plus the matched emission, amplified by stream replication.
  jetstream_write_rate = local.required_eps * 2 * var.jetstream_stream_replicas

  nats_replicas_floor = max(
    var.nats_min_replicas,
    ceil(local.core_delivery_rate / var.nats_msgs_per_server),
    ceil(local.jetstream_write_rate / var.jetstream_writes_per_server),
  )

  # Rounded up to odd. JetStream elects a leader per stream, and an even
  # membership gains no fault tolerance over the odd one below it while adding
  # a vote to every quorum.
  nats_replicas_total = (
    local.nats_replicas_floor % 2 == 0
    ? local.nats_replicas_floor + 1
    : local.nats_replicas_floor
  )

  # Stream leaders spread across the cluster, so a server holds roughly this
  # share of the total data. Sized per server because the file store is a PVC
  # per pod.
  stream_leaders_total    = var.streams + var.matched_streams
  file_store_bytes_server = ceil(local.jetstream_disk_bytes / local.nats_replicas_total)
  file_store_gib_server   = max(1, ceil(local.file_store_bytes_server / 1073741824))

  # What the file store has to sustain, as opposed to how much it has to hold.
  # These are different constraints and only one of them is capacity: a volume
  # sized for the retention window can still be far too slow for the write
  # rate, which is why the storage class provisions performance explicitly
  # rather than inheriting it from the size.
  #
  # The matched stream dominates the bytes. An emission carries the entire cut
  # trip rather than a delta — the property that lets competing solves resolve
  # by revision — so it costs an order of magnitude more per message than the
  # raw event that produced it.
  jetstream_write_bytes_rate = (
    (local.required_eps * var.raw_event_bytes + local.required_eps * var.matched_event_bytes)
    * var.jetstream_stream_replicas
  )

  jetstream_write_mib_server = ceil(
    local.jetstream_write_bytes_rate / local.nats_replicas_total / 1048576
  )

  # JetStream appends and fsyncs in batches rather than once per message, so
  # the operation count is well under the message rate. The divisor is the
  # softest number here.
  jetstream_iops_server = ceil(
    (local.required_eps * 2 * var.jetstream_stream_replicas)
    / local.nats_replicas_total / var.jetstream_messages_per_write
  )

  # Fault tolerance is a property of how the servers are spread, not of how
  # many there are. A node may hold at most half the cluster short of one, or
  # losing it costs quorum — so the pool needs at least this many nodes for the
  # hard spread constraint to be satisfiable.
  nats_spread_nodes = ceil(local.nats_replicas_total / 2)

  # --- Valkey -------------------------------------------------------------

  # One fleet, hashed by vehicle, sized from the total command rate because
  # every primary is interchangeable. Deliberately off the geohash hierarchy:
  # the keyspace is `vehicle:<id>:positions`, and partitioning it
  # geographically would break the trip continuity it exists to provide.
  valkey_ops_rate        = local.required_eps * var.valkey_ops_per_event
  valkey_primaries_total = max(1, ceil(local.valkey_ops_rate / var.valkey_ops_per_primary))
  valkey_pods_total      = local.valkey_primaries_total * (1 + var.valkey_replicas_per_primary)

  single_valkey_ceiling_eps = var.valkey_ops_per_primary / var.valkey_ops_per_event

  valkey_client_is_sufficient = (
    var.valkey_client_mode == "pooled-hash" || local.valkey_primaries_total == 1
  )

  # --- Telemetry ----------------------------------------------------------

  span_rate          = local.required_eps * var.spans_per_event * var.telemetry_sample_ratio
  collector_replicas = max(1, ceil(local.span_rate / var.collector_spans_per_replica))

  # --- Verdict ------------------------------------------------------------

  meets_target = (
    local.matcher_capacity_eps >= local.required_eps
    && local.orchestrator_capacity_eps >= local.required_eps
    && local.fleet_fits_the_partition_space
    && local.raw_streams_ok
    && local.matched_streams_ok
    && local.valkey_client_is_sufficient
    && var.shard_precision == var.binary_shard_precision
  )

  # --- The design target --------------------------------------------------

  # What the wire-law quantities would have to survive at the endpoint, not at
  # today's load. `streams` and `partitions` are pinned for the life of the
  # deployment, so they are the only numbers that must be right now: every
  # other line here is a variable change away.
  design_required_eps = var.design_target_eps * (1 + var.headroom_ratio)

  design = {
    target_eps        = var.design_target_eps
    required_eps      = local.design_required_eps
    writes_per_stream = local.design_required_eps / var.streams
    streams_required  = ceil(local.design_required_eps / var.jetstream_writes_per_stream)
    streams_ok        = (local.design_required_eps / var.streams) <= var.jetstream_writes_per_stream

    fleet_required   = ceil(local.design_required_eps / local.profile.orchestrator_eps)
    fleet_fits       = ceil(local.design_required_eps / local.profile.orchestrator_eps) <= var.partitions
    matcher_pods     = local.shard_count * max(1, ceil((local.design_required_eps / local.shard_count) / local.profile.matcher_eps))
    valkey_primaries = max(1, ceil((local.design_required_eps * var.valkey_ops_per_event) / var.valkey_ops_per_primary))

    matched_streams_required = ceil(local.design_required_eps / var.jetstream_writes_per_stream)
  }

  # --- Node packing -------------------------------------------------------

  # GKE reserves CPU and memory on published sliding scales, both large enough
  # at these node sizes that ignoring them over-commits every pool.
  allocatable = {
    for name, m in var.machines : name => {
      machine_type = m.machine_type
      vcpu         = m.vcpu
      memory_gib   = m.memory_gib

      # 6% of core 1, 1% of core 2, 0.5% of cores 3-4, 0.25% of the rest.
      cpu_millis = floor(
        (m.vcpu * 1000)
        -(60
          + 10 * min(1, max(0, m.vcpu - 1))
          + 5 * min(2, max(0, m.vcpu - 2))
        + 2.5 * max(0, m.vcpu - 4))
        -var.daemonset_cpu_millis
      )

      # 25% of the first 4GiB, 20% of the next 4, 10% of the next 8, 6% of the
      # next 112, 2% beyond, plus the 100MiB eviction threshold.
      memory_mib = floor(
        (m.memory_gib * 1024)
        -(min(4, m.memory_gib) * 1024 * 0.25
          + min(4, max(0, m.memory_gib - 4)) * 1024 * 0.20
          + min(8, max(0, m.memory_gib - 8)) * 1024 * 0.10
          + min(112, max(0, m.memory_gib - 16)) * 1024 * 0.06
          + max(0, m.memory_gib - 128) * 1024 * 0.02
        + 100)
        -var.daemonset_memory_mib
      )

      pods = var.max_pods_per_node - var.daemonset_pods_per_node
    }
  }

  # Pools are sized from pod *shapes*, not aggregate demand. Aggregate CPU over
  # node CPU permits fractional pods: a 2000m pod on a 31250m node fits 15
  # times, not 15.6, so 1042 of them need 70 nodes and not 67.
  # `count` is the steady state, which sets the autoscaler floor. `count_max`
  # is what the pool must be able to grow to, which sets the ceiling — for
  # matchers that is the HPA's, so a hot shard finds nodes rather than leaving
  # replicas Pending, without paying for them while traffic is even.
  shapes = {
    matcher = [{
      name       = "matcher"
      count      = local.matcher_pods
      count_max  = local.shard_count * local.matcher_replicas_max
      pods       = 1
      cpu_millis = local.profile.matcher_cpu_millis
      memory_mib = local.profile.matcher_memory_mib
    }]

    # One shape now: the historian this pipeline absorbed is gone, and the
    # orchestrator fleet is the only thing here. It has no autoscaler — the
    # fleet size is wire-adjacent, since resizing re-slices the partition
    # space — so its floor and ceiling are the same.
    pipeline = [{
      name       = "orchestrator"
      count      = local.fleet
      count_max  = local.fleet
      pods       = 1
      cpu_millis = local.profile.orchestrator_cpu_millis
      memory_mib = local.profile.orchestrator_memory_mib
    }]

    infra = [
      {
        name       = "nats"
        count      = local.nats_replicas_total
        count_max  = local.nats_replicas_total
        pods       = 1
        cpu_millis = var.nats_cpu_millis
        memory_mib = var.nats_memory_mib
      },
      {
        name       = "valkey"
        count      = local.valkey_pods_total
        count_max  = local.valkey_pods_total
        pods       = 1
        cpu_millis = var.valkey_cpu_millis
        memory_mib = var.valkey_memory_mib
      },
    ]

    system = [
      {
        name       = "otel-collector"
        count      = local.collector_replicas
        count_max  = local.collector_replicas
        pods       = 1
        cpu_millis = var.collector_cpu_millis
        memory_mib = var.collector_memory_mib
      },
      {
        # prometheus, grafana, alertmanager, the operator, kube-state-metrics
        # and the dashboard sidecars, as one lumped allowance.
        name       = "observability"
        count      = 1
        count_max  = 1
        pods       = 6
        cpu_millis = 4000
        memory_mib = 16384
      },
    ]
  }

  # How many of each shape fit one node, bounded by whichever of CPU, memory or
  # the pod budget binds first.
  packing = {
    for pool, shapes in local.shapes : pool => [
      for s in shapes : merge(s, {
        per_node = min(
          floor(local.allocatable[pool].cpu_millis / s.cpu_millis),
          floor(local.allocatable[pool].memory_mib / s.memory_mib),
          floor(local.allocatable[pool].pods / s.pods),
        )
      })
    ]
  }

  # A shape that fits zero times per node stays Pending forever while the
  # autoscaler adds nodes that cannot host it.
  unschedulable_shapes = flatten([
    for pool, shapes in local.packing : [
      for s in shapes : "${pool}/${s.name}" if s.count > 0 && s.per_node < 1
    ]
  ])

  # Shapes are summed rather than bin-packed together, so a mixed pool carries
  # slack. That is the conservative direction, and the pools that dominate the
  # fleet hold one shape each.
  node_counts_packed = {
    for pool, shapes in local.packing : pool => max(1, sum([
      for s in shapes : s.per_node < 1 ? s.count : ceil(s.count / s.per_node)
    ]))
  }

  # Packing alone would happily put every NATS server on one large node, which
  # is efficient and destroys the cluster's fault tolerance. The infra pool
  # therefore has a floor independent of how well its pods pack.
  node_counts = merge(local.node_counts_packed, {
    infra = max(local.node_counts_packed["infra"], local.nats_spread_nodes)
  })

  node_counts_max = merge({
    for pool, shapes in local.packing : pool => max(1, sum([
      for s in shapes : s.per_node < 1 ? s.count_max : ceil(s.count_max / s.per_node)
    ]))
    }, {
    infra = max(local.node_counts_packed["infra"], local.nats_spread_nodes)
  })

  demand = {
    for pool, shapes in local.shapes : pool => {
      pods       = sum([for s in shapes : s.count * s.pods])
      cpu_millis = sum([for s in shapes : s.count * s.cpu_millis])
      memory_mib = sum([for s in shapes : s.count * s.memory_mib])
    }
  }

  # The ceiling leaves room for whatever autoscaling the pool's workloads do,
  # for the headroom burst, and for rollout surge — where a replacement pod
  # schedules before the old one goes.
  pools = {
    for pool, nodes in local.node_counts : pool => {
      machine_type    = local.allocatable[pool].machine_type
      node_count      = nodes
      min_node_count  = nodes
      max_node_count  = ceil(local.node_counts_max[pool] * (1 + var.headroom_ratio)) + 1
      allocatable     = local.allocatable[pool]
      demand          = local.demand[pool]
      packing         = local.packing[pool]
      pods_per_node   = ceil(local.demand[pool].pods / nodes)
      cpu_utilisation = local.demand[pool].cpu_millis / (nodes * local.allocatable[pool].cpu_millis)
    }
  }

  total_nodes = sum(values(local.node_counts))
  total_vcpu  = sum([for pool, n in local.node_counts : n * var.machines[pool].vcpu])
  total_pods  = sum([for pool, d in local.demand : d.pods])

  # A /16 gives 65k addresses, so at 110 pods per node it caps the fleet near
  # 256 nodes.
  pod_ips_required = local.total_nodes * var.max_pods_per_node
}
