# The sizing model. Pure arithmetic, no resources, so it plans without
# credentials and `tofu test` exercises it directly. ../../ARCHITECTURE.md
# carries the reasoning; this file carries the numbers.
#
# Two levers, and only two:
#
#   horizontal  more shards, via `shard_precision`
#   vertical    more workers per shard, via `vertical_profile`
#
# Replica count is not a third. Matcher and orchestrator subscriptions are
# plain ephemeral subscribes with no queue group, so a second replica on a
# subject repeats the work rather than sharing it.
#
# Geohash prefixes nest, which gives a partition level for free:
#
#   shard  (shard_precision)  one matcher, one orchestrator
#   cell   (cell_precision)   one historian subscription, one Helm release
#
# Both are subject tokens — `<prefix>.<verb>.<cell>.<rest>` — so a wildcard
# addresses either level. Neither NATS nor Valkey follows that hierarchy.

locals {
  profile     = var.profiles[var.vertical_profile]
  shard_count = length(var.shards)

  # --- Cells --------------------------------------------------------------

  shard_cell = { for s in var.shards : s => substr(s, 0, var.cell_precision) }
  cells      = distinct(values(local.shard_cell))
  cell_count = length(local.cells)

  shards_by_cell = {
    for cell in local.cells : cell => [
      for s in var.shards : s if local.shard_cell[s] == cell
    ]
  }

  # The shard's geohash past the cell prefix, kept as its own subject token so
  # `<cell>.*` matches a cell and `<cell>.<rest>` matches one shard.
  shard_suffix = {
    for s in var.shards : s => substr(s, var.cell_precision, var.shard_precision - var.cell_precision)
  }

  subjects = {
    for s in var.shards : s => {
      cell     = local.shard_cell[s]
      suffix   = local.shard_suffix[s]
      position = "${var.subject_prefix}.position.${local.shard_cell[s]}.${local.shard_suffix[s]}"
      match    = "${var.subject_prefix}.match.${local.shard_cell[s]}.${local.shard_suffix[s]}"
      matched  = "${var.subject_prefix}.matched.${local.shard_cell[s]}.${local.shard_suffix[s]}"
    }
  }

  historian_subjects = (
    var.historian_mode == "per-cell"
    ? { for cell in local.cells : cell => "${var.subject_prefix}.position.${cell}.*" }
    : var.historian_mode == "per-shard"
    ? { for s in var.shards : s => local.subjects[s].position }
    : { global = "${var.subject_prefix}.position.>" }
  )

  # Shards outside the declared service region. A shard list generated over the
  # wrong extent otherwise becomes a fleet of idle matchers.
  out_of_coverage = length(var.coverage_cells) == 0 ? [] : [
    for s in var.shards : s
    if !anytrue([for c in var.coverage_cells : startswith(s, c)])
  ]

  # --- Throughput ---------------------------------------------------------

  required_eps = var.throughput_target_eps * (1 + var.headroom_ratio)
  capacity_eps = local.shard_count * local.profile.shard_eps

  shards_required = ceil(local.required_eps / local.profile.shard_eps)
  shard_deficit   = max(0, local.shards_required - local.shard_count)

  # Precision that would close the deficit, capped at the geohash maximum. If
  # 12 is still short the answer is a bigger profile, and `meets_target` says so.
  precision_levels_needed = local.shard_deficit == 0 ? 0 : ceil(
    log(local.shards_required / local.shard_count, var.shard_fanout_per_precision)
  )
  recommended_precision = min(12, var.shard_precision + local.precision_levels_needed)

  # --- Pods ---------------------------------------------------------------

  # One each per shard, always: each owns a subject exclusively.
  matcher_pods      = local.shard_count
  orchestrator_pods = local.shard_count

  # Uniform load per shard is optimistic — shards are geographic and traffic is
  # not — but the busiest cell is what sets the historian ceiling, so this is
  # measured per cell rather than on the mean.
  cell_eps = {
    for cell, shards in local.shards_by_cell :
    cell => length(shards) * local.profile.shard_eps
  }

  # Subject partitioning divides historian work with no duplication: each pod
  # owns a disjoint slice. A queue group additionally lets several pods share
  # one slice, so replicas can follow a cell's load.
  historian_by_cell = {
    for cell, eps in local.cell_eps :
    cell => var.historian_queue_group ? max(1, ceil(eps / var.historian_eps_per_pod)) : 1
  }

  historian_pods = (
    var.historian_mode == "per-cell" ? sum(values(local.historian_by_cell))
    : var.historian_mode == "per-shard" ? local.shard_count
    : var.global_historian_replicas
  )

  historian_load = {
    for cell, eps in local.cell_eps :
    cell => var.historian_mode == "per-cell" ? eps / local.historian_by_cell[cell] : (
      var.historian_mode == "per-shard" ? local.profile.shard_eps : local.capacity_eps
    )
  }

  historian_eps_per_pod = local.historian_pods > 0 ? local.capacity_eps / local.historian_pods : 0

  saturated_historians = [
    for cell, load in local.historian_load : cell if load > var.historian_eps_per_pod
  ]

  historian_is_saturated = length(local.saturated_historians) > 0

  # --- NATS ---------------------------------------------------------------

  # One cluster for the whole deployment, sized from the total delivery rate and
  # independent of `cell_precision`. A core NATS server forwards a message only
  # to routes with matching subscription interest, so clustering already
  # partitions by subject and a cluster per cell buys nothing. See
  # ../../ARCHITECTURE.md.
  nats_delivery_rate = local.required_eps * var.nats_hops

  nats_replicas_total = max(
    var.nats_min_replicas,
    ceil(local.nats_delivery_rate / var.nats_msgs_per_server)
  )

  # Every server routes to every other, so connections grow quadratically. This
  # is the number that decides when a single cluster stops being reasonable;
  # gateways and a supercluster are the documented answer past that point.
  nats_route_connections = local.nats_replicas_total * (local.nats_replicas_total - 1) / 2

  cell_plan = {
    for cell, shards in local.shards_by_cell : cell => {
      shards = shards
      eps    = local.cell_eps[cell]
    }
  }

  # --- Valkey -------------------------------------------------------------

  # One fleet, hashed by vehicle, sized from the total command rate because
  # every primary is interchangeable. Deliberately off the geohash hierarchy:
  # the keyspace is `vehicle:<id>:positions`, and partitioning it geographically
  # would break the trip continuity it exists to provide.
  valkey_ops_rate        = var.throughput_target_eps * var.valkey_ops_per_event
  valkey_primaries_total = max(1, ceil(local.valkey_ops_rate / var.valkey_ops_per_primary))
  valkey_pods_total      = local.valkey_primaries_total * (1 + var.valkey_replicas_per_primary)

  # Also the whole deployment's ceiling when clients cannot address more than
  # one primary.
  single_valkey_ceiling_eps = var.valkey_ops_per_primary / var.valkey_ops_per_event

  valkey_client_is_sufficient = (
    var.valkey_client_mode == "pooled-hash" || local.valkey_primaries_total == 1
  )

  # --- Verdict ------------------------------------------------------------

  meets_target = (
    local.capacity_eps >= local.required_eps
    && local.valkey_client_is_sufficient
    && !local.historian_is_saturated
  )

  span_rate          = var.throughput_target_eps * var.spans_per_event * var.telemetry_sample_ratio
  collector_replicas = max(1, ceil(local.span_rate / var.collector_spans_per_replica))

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
  shapes = {
    matcher = [{
      name       = "matcher"
      count      = local.matcher_pods
      pods       = 1
      cpu_millis = local.profile.matcher_cpu_millis
      memory_mib = local.profile.matcher_memory_mib
    }]

    # Two shapes rather than a per-shard pair: with per-cell historians there
    # are far fewer historians than orchestrators, so pairing them would size
    # the pool for pods that do not exist.
    pipeline = [
      {
        name       = "orchestrator"
        count      = local.orchestrator_pods
        pods       = 1
        cpu_millis = local.profile.orchestrator_cpu_millis
        memory_mib = local.profile.orchestrator_memory_mib
      },
      {
        name  = "historian"
        count = local.historian_pods
        pods  = 1
        # A per-cell historian carries a whole cell's write rate, above the
        # per-shard figure the profile describes.
        cpu_millis = local.profile.historian_cpu_millis * (var.historian_mode == "per-cell" ? 4 : 1)
        memory_mib = local.profile.historian_memory_mib * (var.historian_mode == "per-cell" ? 4 : 1)
      },
    ]

    infra = [
      {
        name       = "nats"
        count      = local.nats_replicas_total
        pods       = 1
        cpu_millis = var.nats_cpu_millis
        memory_mib = var.nats_memory_mib
      },
      {
        name       = "valkey"
        count      = local.valkey_pods_total
        pods       = 1
        cpu_millis = var.valkey_cpu_millis
        memory_mib = var.valkey_memory_mib
      },
    ]

    system = [
      {
        name       = "otel-collector"
        count      = local.collector_replicas
        pods       = 1
        cpu_millis = var.collector_cpu_millis
        memory_mib = var.collector_memory_mib
      },
      {
        # prometheus, grafana, alertmanager, the operator, kube-state-metrics
        # and the dashboard sidecars, as one lumped allowance.
        name       = "observability"
        count      = 1
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
  node_counts = {
    for pool, shapes in local.packing : pool => max(1, sum([
      for s in shapes : s.per_node < 1 ? s.count : ceil(s.count / s.per_node)
    ]))
  }

  demand = {
    for pool, shapes in local.shapes : pool => {
      pods       = sum([for s in shapes : s.count * s.pods])
      cpu_millis = sum([for s in shapes : s.count * s.cpu_millis])
      memory_mib = sum([for s in shapes : s.count * s.memory_mib])
    }
  }

  # The ceiling leaves room for the headroom burst and for rollout surge, where
  # a shard's replacement pod schedules before the old one goes.
  pools = {
    for pool, nodes in local.node_counts : pool => {
      machine_type    = local.allocatable[pool].machine_type
      node_count      = nodes
      min_node_count  = nodes
      max_node_count  = ceil(nodes * (1 + var.headroom_ratio)) + 1
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
  # 595 nodes.
  pod_ips_required = local.total_nodes * var.max_pods_per_node
}
