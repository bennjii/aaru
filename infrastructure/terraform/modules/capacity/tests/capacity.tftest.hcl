# The sizing model is pure arithmetic, so it is testable outright: no
# providers, no credentials, no cluster. Run with `tofu test` from this module.
#
# Each run pins a number something downstream depends on. If a calibration
# changes on purpose the expected value changes with it; if one changes by
# accident this catches it before a plan does.

variables {
  machines = {
    matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
    pipeline = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 64 }
    infra    = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
    system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
  }

  shard_precision = 6
  cell_precision  = 2
}

# A CONUS-shaped precision-6 shard list: 20 cells at precision 2, each with 8
# groups at precision 3, each with 7 shards. 1120 in total.
#
# It comes from a module because a test `variables` block cannot call
# functions, and `apply` because a plan would leave the output unknown.
run "shards" {
  command = apply

  module {
    source = "./tests/setup"
  }

  variables {
    cells = ["9q", "9r", "9t", "9u", "9v", "9w", "9x", "9y", "9z", "c2",
    "c8", "c9", "cb", "dj", "dn", "dp", "dq", "dr", "f0", "f2"]
  }

  assert {
    condition     = length(output.shards) == 1120
    error_message = "The fixture must generate 1120 shards, got ${length(output.shards)}."
  }
}

# GKE's published reservation formulae, checked against a shape easy to verify
# by hand. Every pool size divides by these.
run "gke_allocatable_matches_published_reservations" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 36000
  }

  # c4-highcpu-32: 6% of core 1 + 1% of core 2 + 0.5% of cores 3-4
  # + 0.25% of the remaining 28 = 150m. Less the 600m DaemonSet allowance.
  assert {
    condition     = output.pools["matcher"].allocatable.cpu_millis == 31250
    error_message = "CPU reservation drifted: got ${output.pools["matcher"].allocatable.cpu_millis}m, expected 31250m."
  }

  # 64 GiB = 65536 MiB. Reserved = 25% of 4GiB (1024) + 20% of 4GiB (819.2)
  # + 10% of 8GiB (819.2) + 6% of 48GiB (2949.12) + 100 MiB eviction.
  # Less the 1200 MiB DaemonSet allowance.
  assert {
    condition     = output.pools["matcher"].allocatable.memory_mib == 58624
    error_message = "Memory reservation drifted: got ${output.pools["matcher"].allocatable.memory_mib} MiB, expected 58624 MiB."
  }
}

# The levels must nest, and be derived from the shard list rather than
# configured independently.
run "levels_nest_by_geohash_prefix" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
  }

  assert {
    condition     = output.shard_count == 1120
    error_message = "Expected 1120 shards, got ${output.shard_count}."
  }

  assert {
    condition     = length(output.cells) == 20
    error_message = "Expected 20 cells at precision 2, got ${length(output.cells)}."
  }

  assert {
    condition     = sum([for c, shards in output.shards_by_cell : length(shards)]) == 1120
    error_message = "Shards were lost or duplicated when grouping into cells."
  }

  # The two-phase subject: cell and remaining geohash as separate tokens, so a
  # wildcard can address either level. With the geohash in one token neither
  # `<cell>.*` nor a per-cell historian is expressible.
  assert {
    condition     = output.subjects["9q0000"].position == "events.position.9q.0000"
    error_message = "Expected events.position.9q.0000, got ${output.subjects["9q0000"].position}."
  }

  assert {
    condition     = output.subjects["9q0000"].match == "events.match.9q.0000"
    error_message = "Expected events.match.9q.0000, got ${output.subjects["9q0000"].match}."
  }

  # Cell plus suffix must reconstruct the shard exactly, or a matcher would
  # subscribe to a subject nothing publishes to.
  assert {
    condition = alltrue([
      for shard, s in output.subjects : "${s.cell}${s.suffix}" == shard
    ])
    error_message = "A shard's cell and suffix do not reconstruct its geohash."
  }

  # A per-cell historian covers a whole cell with one subscription.
  assert {
    condition     = output.historian_subjects["9q"] == "events.position.9q.*"
    error_message = "Expected the per-cell historian on events.position.9q.*, got ${output.historian_subjects["9q"]}."
  }
}

# The chart's shipped Sydney default: six precision-4 shards is a development
# footprint, and asserting how far short it falls is the point.
run "sydney_precision_4_is_short_of_5m" {
  command = plan

  variables {
    throughput_target_eps = 5000000
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    shard_precision       = 4
    cell_precision        = 2
  }

  assert {
    condition     = output.capacity_eps == 36000
    error_message = "Expected 36000 evt/s from 6 standard shards, got ${output.capacity_eps}."
  }

  assert {
    condition     = output.meets_target == false
    error_message = "6 shards must not be reported as meeting a 5M evt/s target."
  }

  # ceil(5000000 * 1.25 / 6000).
  assert {
    condition     = output.shards_required == 1042
    error_message = "Expected 1042 shards required, got ${output.shards_required}."
  }

  # ceil(log(1042/6) / log(8)) = 3 levels above precision 4.
  assert {
    condition     = output.recommended_precision == 7
    error_message = "Expected precision 7 to close the deficit, got ${output.recommended_precision}."
  }
}

# The horizontal lever. 1120 shards clears the target on throughput, but the
# Valkey topology still has to hold — see the next two runs.
run "shard_count_reaches_5m_on_throughput" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
  }

  # 1120 x 6000 = 6.72M >= 6.25M required.
  assert {
    condition     = output.capacity_eps == 6720000
    error_message = "Expected 6720000 evt/s, got ${output.capacity_eps}."
  }

  assert {
    condition     = output.shard_deficit == 0
    error_message = "1120 standard shards should have no shard deficit, got ${output.shard_deficit}."
  }

  # One matcher and one orchestrator per shard is structural: neither can be
  # replicated without duplicating work.
  assert {
    condition     = output.replicas.matcher == 1120 && output.replicas.orchestrator == 1120
    error_message = "Matcher and orchestrator must be one per shard."
  }

  # A per-cell historian needs one pod per cell, not one per shard. That is the
  # whole return on the two-phase subject: 20 pods instead of 1120.
  assert {
    condition     = output.replicas.historian == 20
    error_message = "per-cell historian mode should yield one historian per cell, got ${output.replicas.historian}."
  }
}

# The Valkey fleet is sized from the total command rate, because every primary
# is interchangeable. 5M evt/s x 2 commands = 10M ops/s, at the 500k per
# primary measured in the 1B RPS cluster run.
run "valkey_fleet_is_sized_from_total_command_rate" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    valkey_client_mode    = "pooled-hash"
  }

  assert {
    condition     = output.valkey.ops_rate == 10000000
    error_message = "Expected 10M ops/s, got ${output.valkey.ops_rate}."
  }

  # ceil(10000000 / 500000). The old model demanded 84 by assuming 120k.
  assert {
    condition     = output.valkey.primaries == 20
    error_message = "Expected 20 primaries at 500k ops/s each, got ${output.valkey.primaries}."
  }

  assert {
    condition     = output.valkey.pods == 40
    error_message = "Expected 40 Valkey pods with one replica each, got ${output.valkey.pods}."
  }

  # 500000 / 2 commands per event.
  assert {
    condition     = output.single_valkey_ceiling_eps == 250000
    error_message = "Expected a 250000 evt/s single-primary ceiling, got ${output.single_valkey_ceiling_eps}."
  }

  assert {
    condition     = output.valkey_client_prerequisite == ""
    error_message = "pooled-hash must not report a client prerequisite."
  }
}

# The client, not the fleet, is what caps a `single` deployment. Adding pods
# cannot help, so the model must say that rather than report a pod shortfall.
run "single_client_mode_caps_the_deployment" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    valkey_client_mode    = "single"
  }

  assert {
    condition     = output.meets_target == false
    error_message = "A single-connection client must not be reported as meeting 5M evt/s."
  }

  # Throughput and shard count are both fine; only the client is not.
  assert {
    condition     = output.shard_deficit == 0
    error_message = "Must fail on the Valkey client, not on shard count."
  }

  assert {
    condition     = strcontains(output.valkey_client_prerequisite, "pooled-hash")
    error_message = "The prerequisite must name the fix, got: ${output.valkey_client_prerequisite}."
  }
}

# Vertical scaling on Valkey: a faster primary needs fewer of them. This is the
# io-threads lever, and it moves the fleet size directly.
run "faster_primaries_shrink_the_fleet" {
  command = plan

  variables {
    shards                 = run.shards.shards
    throughput_target_eps  = 5000000
    valkey_ops_per_primary = 1000000
  }

  assert {
    condition     = output.valkey.primaries == 10
    error_message = "Expected 10 primaries at 1M ops/s each, got ${output.valkey.primaries}."
  }
}

# Halving commands per event halves the fleet. This is the per-vehicle cache in
# the orchestrator, and it is the cheapest capacity in the system.
run "removing_the_read_halves_the_fleet" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    valkey_ops_per_event  = 1
  }

  assert {
    condition     = output.valkey.primaries == 10
    error_message = "Expected 10 primaries at one command per event, got ${output.valkey.primaries}."
  }
}

# One cluster, sized from the delivery rate. A core NATS server forwards a
# message only to routes with matching subscription interest, so clustering
# already partitions by subject and a per-cell split buys nothing.
run "nats_is_one_cluster_sized_by_delivery_rate" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
  }

  # 6.25M required x 4 deliveries per event.
  assert {
    condition     = output.nats.delivery_rate == 25000000
    error_message = "Expected 25M deliveries/s, got ${output.nats.delivery_rate}."
  }

  # ceil(25000000 / 1000000).
  assert {
    condition     = output.nats.replicas_total == 25
    error_message = "Expected 25 NATS servers, got ${output.nats.replicas_total}."
  }

  # Full mesh, N(N-1)/2. The number to watch before growing the cluster.
  assert {
    condition     = output.nats.route_connections == 300
    error_message = "Expected 300 route connections at 25 servers, got ${output.nats.route_connections}."
  }

  # The old per-cell model paid a three-server floor 20 times over.
  assert {
    condition     = output.nats.replicas_total < 60
    error_message = "One cluster must cost less than the 60 servers a cluster per cell did."
  }
}

# The point of the rework. Cells were NATS clusters, so precision 3 meant 160
# clusters and 480 servers carrying identical traffic. Cells are now a subject
# grouping, and the broker does not notice them at all.
run "nats_is_independent_of_cell_layout" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    cell_precision        = 3
  }

  assert {
    condition     = length(output.cells) == 160
    error_message = "Expected 160 cells at precision 3, got ${length(output.cells)}."
  }

  assert {
    condition     = output.nats.replicas_total == 25
    error_message = "NATS must not follow cell count, got ${output.nats.replicas_total} servers across 160 cells."
  }

  # Valkey never followed it either: the keyspace is vehicle-keyed, which is
  # what lets a trip continue across a shard boundary.
  assert {
    condition     = output.valkey.primaries == 20
    error_message = "Valkey must not follow cell count, got ${output.valkey.primaries}."
  }
}

# A slower server needs more of them, and that is the only dial NATS has. The
# default is derived from loopback benchmarks, so this is the knob to move once
# there is a real measurement.
run "slower_servers_grow_the_nats_cluster" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    nats_msgs_per_server  = 500000
  }

  assert {
    condition     = output.nats.replicas_total == 50
    error_message = "Expected 50 servers at 500k deliveries each, got ${output.nats.replicas_total}."
  }
}

# Small deployments sit on the availability floor rather than on throughput.
run "small_deployments_hold_the_availability_floor" {
  command = plan

  variables {
    throughput_target_eps = 10000
    shards                = ["r3gq", "r3gr"]
    shard_precision       = 4
  }

  assert {
    condition     = output.nats.replicas_total == 3
    error_message = "Expected the 3-server floor, got ${output.nats.replicas_total}."
  }
}

# A cell that outgrows one historian must be reported, because the mean stays
# healthy while one pod falls behind.
run "oversized_cells_saturate_their_historian" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    historian_mode        = "per-cell"
    historian_queue_group = false
  }

  # 56 shards x 6000 = 336000 evt/s into one pod that sustains 150000.
  assert {
    condition     = length(output.saturated_historian_cells) == 20
    error_message = "All 20 cells should saturate one historian, got ${length(output.saturated_historian_cells)}."
  }

  assert {
    condition     = output.historian_saturated && output.meets_target == false
    error_message = "A saturated historian must fail the target."
  }
}

# The queue group lets historian count follow load while cells stay coarse, so
# the fleet does not gain a Helm release per cell to feed the archiver.
run "queue_group_scales_historians_without_shrinking_cells" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    historian_mode        = "per-cell"
    historian_queue_group = true
  }

  # ceil(336000 / 150000) = 3 replicas per cell.
  assert {
    condition     = alltrue([for cell, n in output.historian_by_cell : n == 3])
    error_message = "Expected 3 historians per cell, got ${jsonencode(output.historian_by_cell)}."
  }

  assert {
    condition     = output.replicas.historian == 60
    error_message = "Expected 60 historians across 20 cells, got ${output.replicas.historian}."
  }

  assert {
    condition     = length(output.saturated_historian_cells) == 0
    error_message = "Queue-grouped historians should not saturate."
  }

  assert {
    condition     = output.meets_target
    error_message = "Queue-grouped historians should meet the target."
  }

  # Historians scaled; the broker did not, because it is sized by delivery rate
  # and the delivery rate did not change.
  assert {
    condition     = output.nats.replicas_total == 25
    error_message = "NATS must not grow when historians do, got ${output.nats.replicas_total}."
  }
}

# The vertical lever: `large` lifts per-shard throughput from 6k to 16k, so the
# same target needs ~2.7x fewer shards.
run "vertical_scaling_reduces_shard_requirement" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    vertical_profile      = "large"
  }

  assert {
    condition     = output.shards_required == 391
    error_message = "Expected 391 large shards required, got ${output.shards_required}."
  }

  assert {
    condition     = output.profile.shard_eps == 16000
    error_message = "large profile should sustain 16000 evt/s per shard."
  }
}

# Restricting to a service region is what keeps the shard count sane. A shard
# generated outside it would otherwise become an idle matcher.
run "coverage_allowlist_catches_out_of_region_shards" {
  command = plan

  variables {
    throughput_target_eps = 100000
    shards                = ["9q0000", "9q0001", "r3gq00"]
    shard_precision       = 6
    coverage_cells        = ["9", "c", "d", "f"]
  }

  assert {
    condition     = length(output.out_of_coverage_shards) == 1 && output.out_of_coverage_shards[0] == "r3gq00"
    error_message = "Expected the Sydney shard to fall outside CONUS coverage, got ${jsonencode(output.out_of_coverage_shards)}."
  }
}

run "coverage_allowlist_passes_when_all_shards_nest" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
    coverage_cells        = ["9", "c", "d", "f"]
  }

  assert {
    condition     = length(output.out_of_coverage_shards) == 0
    error_message = "CONUS shards should all nest under CONUS coverage, got ${jsonencode(output.out_of_coverage_shards)}."
  }
}

# Unsampled telemetry at target rate is what the default sample ratio guards
# against: 40M spans/s needs a collector fleet larger than the pipeline.
run "unsampled_telemetry_is_visibly_infeasible" {
  command = plan

  variables {
    shards                 = run.shards.shards
    throughput_target_eps  = 5000000
    telemetry_sample_ratio = 1.0
  }

  assert {
    condition     = output.telemetry.collector_replicas == 800
    error_message = "Unsampled 5M evt/s should need 800 collectors, got ${output.telemetry.collector_replicas}."
  }
}

run "sampled_telemetry_fits_one_collector" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
  }

  # 5M x 8 spans x 0.001 = 40k spans/s, at 50k per replica.
  assert {
    condition     = output.telemetry.collector_replicas == 1
    error_message = "Expected 1 collector at the default sample ratio, got ${output.telemetry.collector_replicas}."
  }
}

# Node packing: pods are integral, so a pool must never be sized by dividing
# aggregate demand.
run "pools_are_never_overcommitted" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 5000000
  }

  assert {
    condition = alltrue([
      for name, p in output.pools :
      p.node_count * p.allocatable.cpu_millis >= p.demand.cpu_millis
    ])
    error_message = "A pool was sized below its aggregate CPU request."
  }

  assert {
    condition = alltrue([
      for name, p in output.pools :
      p.node_count * p.allocatable.memory_mib >= p.demand.memory_mib
    ])
    error_message = "A pool was sized below its aggregate memory request."
  }

  assert {
    condition = alltrue([
      for name, p in output.pools :
      p.node_count * p.allocatable.pods >= p.demand.pods
    ])
    error_message = "A pool was sized below its pod count."
  }

  # The invariant the aggregate assertions miss.
  assert {
    condition = alltrue(flatten([
      for name, p in output.pools : [
        for s in p.packing : s.per_node * p.node_count >= s.count
      ]
    ]))
    error_message = "A pod shape does not fit its allocated node count under integral packing."
  }

  assert {
    condition     = length(output.unschedulable_shapes) == 0
    error_message = "Shapes too large for their node: ${join(", ", output.unschedulable_shapes)}."
  }

  assert {
    condition = alltrue([
      for name, p in output.pools : p.max_node_count > p.min_node_count
    ])
    error_message = "Every pool needs surge room above its steady state."
  }

  # 1120 matchers requesting 2000m on a c4-highcpu-32 with 31250m allocatable:
  # 15 fit per node (memory would allow 19), so 75 nodes. The aggregate form
  # would have said 72 and under-provisioned by three.
  assert {
    condition     = output.pools["matcher"].node_count == 75
    error_message = "Expected 75 matcher nodes for 1120 standard matchers, got ${output.pools["matcher"].node_count}."
  }

  # The pipeline pool holds two independent shapes now that historians are
  # per-cell rather than per-shard. 1120 orchestrators at 1000m pack 15 to a
  # c4-standard-16 (15290m allocatable) for 75 nodes; 20 per-cell historians at
  # 2000m pack 7 to a node for 3 more.
  assert {
    condition     = output.pools["pipeline"].node_count == 78
    error_message = "Expected 78 pipeline nodes, got ${output.pools["pipeline"].node_count}."
  }

  # 25 NATS servers at 4000m take a c4-standard-8 (7310m allocatable) each;
  # 40 Valkey pods at 2000m pack 3 to a node for 14 more.
  assert {
    condition     = output.pools["infra"].node_count == 39
    error_message = "Expected 39 infra nodes, got ${output.pools["infra"].node_count}."
  }

  # Pod IPs bound the subnet's secondary range, and a /16 caps the fleet near
  # 595 nodes at 110 pods each.
  assert {
    condition     = output.totals.pod_ips_required == output.totals.nodes * 110
    error_message = "Pod IP demand must follow node count times max pods per node."
  }
}

# A profile whose requests exceed its pool's machine must be caught at plan
# time, not left as a permanently Pending pod.
run "detects_shapes_too_large_for_their_node" {
  command = plan

  variables {
    throughput_target_eps = 100000
    shards                = ["9q0000", "9q0001"]
    vertical_profile      = "large"

    machines = {
      matcher  = { machine_type = "c4-highcpu-2", vcpu = 2, memory_gib = 4 }
      pipeline = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 64 }
      infra    = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
      system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
    }
  }

  assert {
    condition     = contains(output.unschedulable_shapes, "matcher/matcher")
    error_message = "A 6000m matcher on a 2-vCPU node must be reported unschedulable, got ${jsonencode(output.unschedulable_shapes)}."
  }
}

run "rejects_duplicate_shards" {
  command = plan

  variables {
    throughput_target_eps = 1000
    shards                = ["r3gq", "r3gq"]
    shard_precision       = 4
  }

  expect_failures = [var.shards]
}

# A shard shorter than its precision would make the subject split read past the
# end of the string, which surfaces as an out-of-range substr rather than as
# the mismatch it is.
run "rejects_shards_that_disagree_with_precision" {
  command = plan

  variables {
    throughput_target_eps = 1000
    shards                = ["r3gq", "r3gqxx"]
    shard_precision       = 4
  }

  expect_failures = [var.shard_precision]
}

# Equal precisions leave the shard token of the subject empty, so a matcher
# would subscribe to something nothing publishes to.
run "rejects_cell_precision_at_or_above_shard_precision" {
  command = plan

  variables {
    throughput_target_eps = 1000
    shards                = ["r3gq"]
    shard_precision       = 4
    cell_precision        = 4
  }

  expect_failures = [var.cell_precision]
}

run "rejects_out_of_range_precision" {
  command = plan

  variables {
    throughput_target_eps = 1000
    shards                = ["r3gq"]
    shard_precision       = 13
  }

  expect_failures = [var.shard_precision]
}

run "rejects_unsampled_ratio_above_one" {
  command = plan

  variables {
    throughput_target_eps  = 1000
    shards                 = ["r3gq"]
    shard_precision        = 4
    telemetry_sample_ratio = 2.0
  }

  expect_failures = [var.telemetry_sample_ratio]
}
