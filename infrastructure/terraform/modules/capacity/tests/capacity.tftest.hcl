# The sizing model is pure arithmetic, so it is testable outright: no
# providers, no credentials, no cluster. Run with `tofu test` from this module.
#
# Each run pins a number something downstream depends on. If a calibration
# changes on purpose the expected value changes with it; if one changes by
# accident this catches it before a plan does.

variables {
  machines = {
    matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
    pipeline = { machine_type = "c4-highcpu-16", vcpu = 16, memory_gib = 32 }
    infra    = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 60 }
    system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 30 }
  }

  shard_precision = 4
}

# 256 precision-4 shards: eight three-character prefixes fanned across the
# geohash alphabet. It comes from a module because a test `variables` block
# cannot call functions, and `apply` because a plan would leave the output
# unknown.
run "shards" {
  command = apply

  module {
    source = "./tests/setup"
  }

  variables {
    prefixes = ["r3g", "r65", "r3f", "r3u", "qd0", "qd1", "r6h", "r3e"]
  }

  assert {
    condition     = length(output.shards) == 256
    error_message = "The fixture must generate 256 shards, got ${length(output.shards)}."
  }
}

# GKE's published reservation formulae, checked against a shape easy to verify
# by hand. Every pool size divides by these.
run "gke_allocatable_matches_published_reservations" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 100000
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

  assert {
    condition     = length(output.unschedulable_shapes) == 0
    error_message = "Every pod shape must fit one node: ${join(", ", output.unschedulable_shapes)}."
  }
}

# The two halves scale on different axes and must not be derived from each
# other: matchers from geography, the fleet from the vehicle partition space.
run "the_two_halves_scale_independently" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  # 800k + 25% = 1M, over 256 shards = 3906 evt/s each, inside one replica's
  # 6000. Replicas are the first lever, so this stays at the floor.
  assert {
    condition     = output.matcher.replicas == 1
    error_message = "Expected 1 replica per shard at 3906 evt/s mean, got ${output.matcher.replicas}."
  }

  assert {
    condition     = output.matcher.pods == 256
    error_message = "Expected 256 matcher pods, got ${output.matcher.pods}."
  }

  # The HPA ceiling is what covers a hot shard, since the model can only see
  # the mean.
  assert {
    condition     = output.matcher.replicas_max == 4
    error_message = "Expected an HPA ceiling of 4 replicas, got ${output.matcher.replicas_max}."
  }

  # 1M / 40000 per pod = 25, snapped up to 32 so every pod takes exactly 32
  # partitions. The fleet is independent of the shard count entirely.
  assert {
    condition     = output.fleet.size == 32
    error_message = "Expected a fleet of 32, got ${output.fleet.size}."
  }

  assert {
    condition     = output.fleet.partitions_per_pod == 32
    error_message = "Expected 32 partitions per pod, got ${output.fleet.partitions_per_pod}."
  }
}

# The orchestrator is an I/O scheduler with a compute cost, and the two bind
# separately. Fusing them into one rate hid a concurrency shortfall as a
# hardware requirement — the shipped 64 workers reach 8k evt/s against cores
# worth 40k, so a pod sized for the cores could never use them.
run "the_orchestrator_reports_which_ceiling_binds" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  # 512 workers over an 8 ms round trip.
  assert {
    condition     = output.fleet.io_eps == 64000
    error_message = "Expected 64000 evt/s of concurrency, got ${output.fleet.io_eps}."
  }

  # 2000 millicores at 50 microseconds an event.
  assert {
    condition     = output.fleet.cpu_eps == 40000
    error_message = "Expected 40000 evt/s of compute, got ${output.fleet.cpu_eps}."
  }

  # The lower of the two is the pod's rate, and the profile is deliberately
  # sized so compute is what binds: concurrency is cheap, cores are not.
  assert {
    condition     = output.fleet.eps_per_pod == 40000
    error_message = "The pod's rate must be the lower bound, got ${output.fleet.eps_per_pod}."
  }

  assert {
    condition     = output.fleet.bound == "compute"
    error_message = "Expected the profile to be compute-bound, got ${output.fleet.bound}."
  }

  # Nothing stranded when compute binds: every core bought is reachable.
  assert {
    condition     = output.orchestrator_efficiency_note == ""
    error_message = output.orchestrator_efficiency_note
  }
}

# A profile with the chart's shipped worker count against these cores, to show
# the failure the split exists to catch.
run "a_concurrency_starved_profile_is_called_out" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000

    profiles = {
      starved = {
        matcher_workers    = 10
        matcher_cpu_millis = 2000
        matcher_memory_mib = 3072
        matcher_eps        = 6000

        # The chart default, against a pod sized for far more.
        orchestrator_workers              = 64
        orchestrator_round_trip_ms        = 8
        orchestrator_cpu_micros_per_event = 50
        orchestrator_cpu_millis           = 2000
        orchestrator_memory_mib           = 4096
      }
    }
    vertical_profile = "starved"
  }

  assert {
    condition     = output.fleet.bound == "concurrency"
    error_message = "64 workers over an 8ms round trip must bind on concurrency, got ${output.fleet.bound}."
  }

  # 8k of 40k reachable, so four fifths of every pod's cores are unusable.
  assert {
    condition     = output.fleet.stranded_cpu_millis == 1600
    error_message = "Expected 1600m stranded per pod, got ${output.fleet.stranded_cpu_millis}."
  }

  assert {
    condition     = length(output.orchestrator_efficiency_note) > 0
    error_message = "A concurrency-bound fleet must say so, and say that workers are the fix."
  }
}

# Every fleet size must divide the partition space exactly. The binary gives
# the last pod the remainder, so a non-divisor would quietly make one pod the
# hot one.
run "fleet_snaps_to_a_divisor_of_the_partition_space" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  assert {
    condition     = output.fleet.partitions % output.fleet.size == 0
    error_message = "Fleet of ${output.fleet.size} does not divide ${output.fleet.partitions} partitions."
  }

  assert {
    condition     = output.fleet.size * output.fleet.partitions_per_pod == output.fleet.partitions
    error_message = "Fleet slices do not cover the partition space exactly."
  }

  assert {
    condition     = output.fleet.size >= output.fleet.required
    error_message = "Snapping must round up: fleet ${output.fleet.size} is under the required ${output.fleet.required}."
  }
}

# A profile too small to reach the target within one pod per partition. The
# answer is a bigger profile, and the model must say so rather than plan a
# fleet larger than the space it divides.
run "a_fleet_cannot_exceed_the_partition_space" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 50000000
    vertical_profile      = "small"
  }

  assert {
    condition     = !output.fleet.fits_partitions
    error_message = "50M evt/s at the small profile must exceed the partition space."
  }

  assert {
    condition     = output.fleet.size == output.fleet.partitions
    error_message = "An over-target fleet must pin at the partition count, got ${output.fleet.size}."
  }

  assert {
    condition     = !output.meets_target
    error_message = "A fleet that cannot fit the partition space must not meet the target."
  }
}

# The stream count is the one quantity a migration cannot avoid, so what
# matters is whether the pinned value survives the *design* target rather than
# today's. 5M + 25% over 64 streams is ~98k writes/s each.
run "the_pinned_stream_count_survives_the_design_target" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
    design_target_eps     = 5000000
  }

  assert {
    condition     = output.streams.raw == 64
    error_message = "Expected 64 raw streams pinned, got ${output.streams.raw}."
  }

  assert {
    condition     = output.design_target.streams_ok
    error_message = "64 streams must absorb the design target: ${output.design_target.writes_per_stream} writes/s per stream exceeds the ceiling."
  }

  assert {
    condition     = output.streams.raw_sufficient
    error_message = "64 streams must absorb the current target: ${output.raw_stream_prerequisite}"
  }

  # 1024 partitions over 64 streams: each stream carries 16 subjects and so 16
  # durable consumers, whose state its leader holds.
  assert {
    condition     = output.streams.consumers_per_stream == 16
    error_message = "Expected 16 consumers per stream, got ${output.streams.consumers_per_stream}."
  }

  # Half the streams would put the design target over a single leader's
  # ceiling, which is the reason the pinned value is not 32.
  assert {
    condition     = output.design_target.streams_required > 32
    error_message = "The design target should need more than 32 streams, or the pinned 64 is not justified."
  }
}

# The matched stream is a singleton in `ingest.rs`, which caps the deployment
# well under either target. The model must name it rather than let a cluster
# discover it.
run "the_singleton_matched_stream_is_reported" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  assert {
    condition     = !output.streams.matched_sufficient
    error_message = "One matched stream cannot absorb 1M emissions/s; the model should say so."
  }

  assert {
    condition     = output.streams.matched_streams_required == 7
    error_message = "Expected 7 matched streams required, got ${output.streams.matched_streams_required}."
  }

  assert {
    condition     = length(output.matched_stream_prerequisite) > 0
    error_message = "The prerequisite must carry the remediation, not just a boolean."
  }

  assert {
    condition     = !output.meets_target
    error_message = "A saturated matched stream must fail the verdict."
  }
}

# A rate low enough that one stream is enough end to end, which is the only
# configuration the current binary fully satisfies.
run "a_small_target_is_deliverable_as_the_code_stands" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 100000
    design_target_eps     = 100000
  }

  assert {
    condition     = output.streams.matched_sufficient
    error_message = "125k emissions/s must fit one stream at a 150k ceiling."
  }

  assert {
    condition     = output.meets_target
    error_message = "This shape should be deliverable:\n${output.summary}"
  }
}

# Precision is compiled into the binaries' routing, so a mismatch is not a
# warning — every request would address a subject no matcher serves.
run "a_precision_mismatch_is_fatal" {
  command = plan

  variables {
    shards                 = run.shards.shards
    throughput_target_eps  = 100000
    binary_shard_precision = 6
  }

  assert {
    condition     = !output.meets_target
    error_message = "Deployed precision 4 against a compiled 6 must not meet the target."
  }

  assert {
    condition     = length(output.precision_prerequisite) > 0
    error_message = "The mismatch must be reported with its remediation."
  }
}

# Valkey halved when the orchestrator absorbed the historian: one XADD per
# event, where the old topology also read per event.
run "valkey_costs_one_command_per_event" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  assert {
    condition     = output.valkey.ops_per_event == 1
    error_message = "Expected 1 command per event, got ${output.valkey.ops_per_event}."
  }

  # 1M ops/s over 500k per primary.
  assert {
    condition     = output.valkey.primaries == 2
    error_message = "Expected 2 primaries for 1M ops/s, got ${output.valkey.primaries}."
  }

  assert {
    condition     = output.valkey_client_prerequisite == ""
    error_message = "pooled-hash must satisfy a multi-primary fleet."
  }
}

# `single` reaches one primary, so a fleet larger than one is unreachable and
# adding primaries cannot help.
run "the_single_valkey_client_caps_the_deployment" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
    valkey_client_mode    = "single"
  }

  assert {
    condition     = !output.meets_target
    error_message = "A 2-primary fleet under 'single' must not meet the target."
  }

  assert {
    condition     = length(output.valkey_client_prerequisite) > 0
    error_message = "The client-mode ceiling must be reported."
  }
}

# One shard cannot be scaled without limit: every replica loads that shard's
# whole file, so past a point the geography must be subdivided instead.
run "a_shard_that_cannot_be_scaled_recommends_a_finer_precision" {
  command = plan

  variables {
    shards                         = ["r3gq"]
    throughput_target_eps          = 800000
    max_matcher_replicas_per_shard = 32
  }

  assert {
    condition     = output.shard_deficit > 0
    error_message = "1M evt/s onto one shard must exceed a 32-replica queue group."
  }

  assert {
    condition     = output.recommended_precision > 4
    error_message = "A shard deficit must recommend a finer precision, got ${output.recommended_precision}."
  }
}

# Shards divide the pod count rather than multiplying it, so the two cancel and
# the fleet size is set by throughput alone. What survives the cancellation is
# the per-shard rounding, which finer geography pays more often — the opposite
# of the intuition that fewer, bigger shards mean fewer pods.
run "the_matcher_count_follows_throughput_not_geography" {
  command = plan

  variables {
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    throughput_target_eps = 800000
  }

  # 1M evt/s over 6000 per pod.
  assert {
    condition     = output.matcher.pods_floor == 167
    error_message = "Expected a 167-pod throughput floor, got ${output.matcher.pods_floor}."
  }

  # Six shards land within one pod of it: 28 replicas each is 168.
  assert {
    condition     = output.matcher.pods == 168
    error_message = "Expected 168 pods at 6 shards, got ${output.matcher.pods}."
  }

  assert {
    condition     = output.matcher.rounding_overhead < 1.02
    error_message = "Coarse geography should sit within 2% of the floor, got ${output.matcher.rounding_overhead}x."
  }
}

# The same target over a finer grid costs more pods, not fewer: every shard
# rounds its replicas up to a whole pod, and there are more shards to round.
run "finer_geography_costs_pods_rather_than_saving_them" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  # 256 shards at 3906 evt/s each: one replica apiece, well under a pod's
  # 6000, so more than half of every pod is bought and idle.
  assert {
    condition     = output.matcher.replicas == 1
    error_message = "Expected the one-replica minimum at 256 shards, got ${output.matcher.replicas}."
  }

  assert {
    condition     = output.matcher.pods == 256
    error_message = "Expected 256 pods, got ${output.matcher.pods}."
  }

  assert {
    condition     = output.matcher.pods > output.matcher.pods_floor
    error_message = "A grid finer than the throughput floor must cost pods over it."
  }

  # Past the floor in shard count, the pod count simply follows the shards.
  assert {
    condition     = output.shard_count > output.matcher.pods_floor
    error_message = "This fixture is meant to sit past the throughput floor in shard count."
  }
}

# A matcher's memory is its shard's graph, and the model is a line fitted
# through two real shards rather than a ratio. These pin it against both, which
# is the whole justification for the two-term form: a single expansion factor
# cannot reproduce them, because they disagree about the factor (7.6x and
# 11.8x) while agreeing about the line.
run "the_memory_model_reproduces_the_small_shard" {
  command = plan

  variables {
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    throughput_target_eps = 800000

    # r3gr over Sydney: 21.2 MB on disk, 250 MB resident.
    largest_shard_file_mib = 20.2
  }

  assert {
    condition     = abs(output.matcher.graph_mib - 238) <= 5
    error_message = "The fit must land on r3gr's measured 238 MiB, got ${output.matcher.graph_mib}."
  }

  # With a shard this size the pod is mostly working set, so CPU binds and the
  # graph is effectively free.
  assert {
    condition     = length(regexall("CPU-bound", output.shard_memory_note)) > 0
    error_message = "A precision-4 graph should leave the pool CPU-bound: ${output.shard_memory_note}"
  }
}

run "the_memory_model_reproduces_the_large_shard" {
  command = plan

  variables {
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    throughput_target_eps = 800000

    # r1, the largest in Australia at precision 2: 433 MB on disk, 3.31 GB
    # resident. The same line, twenty times the shard.
    largest_shard_file_mib = 412.9
  }

  assert {
    condition     = abs(output.matcher.graph_mib - 3157) <= 30
    error_message = "The fit must land on r1's measured 3157 MiB, got ${output.matcher.graph_mib}."
  }

  # The graph alone exceeds the flat 3072 MiB the chart used to hardcode, so a
  # matcher holding this shard was never going to start.
  assert {
    condition     = output.matcher.graph_mib > 3072
    error_message = "r1's graph must be shown to exceed the old flat limit, got ${output.matcher.graph_mib}."
  }

  # Every replica loads its shard's whole graph, so the fleet holds the same
  # geography many times over. This is the cost coarse shards actually carry.
  assert {
    condition     = output.matcher.graph_mib_total == output.matcher.pods * output.matcher.graph_mib
    error_message = "Fleet-wide graph memory must be the pod count times the graph."
  }

  # The pod count is identical to the small-shard case: geography never moved
  # it, so a bigger shard multiplies memory with nothing removed to hold it.
  assert {
    condition     = output.matcher.pods == 168
    error_message = "Pod count must not depend on shard size, got ${output.matcher.pods}."
  }
}

# The infra pool must never be packed to the point where losing one node costs
# JetStream its quorum. Bin packing alone would happily do exactly that, since
# a big node fits several brokers.
run "the_infra_pool_keeps_the_brokers_spread" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  # A node may hold at most half the cluster short of one.
  assert {
    condition     = output.nats.spread_nodes >= ceil(output.nats.replicas_total / 2)
    error_message = "A ${output.nats.replicas_total}-server cluster needs at least ${ceil(output.nats.replicas_total / 2)} nodes, got ${output.nats.spread_nodes}."
  }

  assert {
    condition     = output.pools["infra"].min_node_count >= output.nats.spread_nodes
    error_message = "The infra pool floors at ${output.pools["infra"].min_node_count} nodes but the brokers need ${output.nats.spread_nodes} to spread across."
  }

  # Losing the most loaded node must leave a majority behind.
  assert {
    condition     = (output.nats.replicas_total - ceil(output.nats.replicas_total / output.nats.spread_nodes)) > (output.nats.replicas_total / 2)
    error_message = "Losing one infra node would take ${ceil(output.nats.replicas_total / output.nats.spread_nodes)} of ${output.nats.replicas_total} servers, costing quorum."
  }
}

# Capacity and performance are separate constraints on the file store. Sizing
# the volume for the retention window says nothing about whether it can absorb
# the write rate, which is why both are derived.
run "the_file_store_is_sized_for_rate_as_well_as_volume" {
  command = plan

  variables {
    shards                = run.shards.shards
    throughput_target_eps = 800000
  }

  # 1M evt/s of raw at 256B plus 1M emissions at 2KiB, over 5 servers.
  assert {
    condition     = output.jetstream_disk.write_mib_per_server == 440
    error_message = "Expected 440 MiB/s per server, got ${output.jetstream_disk.write_mib_per_server}."
  }

  assert {
    condition     = output.jetstream_disk.iops_per_server == 25000
    error_message = "Expected 25000 IOPS per server, got ${output.jetstream_disk.iops_per_server}."
  }

  # The matched stream is most of the bytes, because an emission carries the
  # whole cut trip rather than a delta. If this ever inverts, the retention
  # policy is no longer what dominates the disk.
  assert {
    condition     = var.matched_event_bytes > var.raw_event_bytes * 4
    error_message = "The disk model assumes emissions dominate ingest by volume."
  }
}

# Cost is derived from the same pod shapes as the node counts, so it cannot
# describe a fleet other than the one that would be built. These pin the
# arithmetic, not the rates — a price change moves the totals and should.
run "cost_follows_the_fleet_that_would_be_built" {
  command = plan

  variables {
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    throughput_target_eps = 800000
  }

  # Nodes are bought whole, so the pool total is count times rate.
  assert {
    condition = (
      output.cost.by_pool["matcher"]
      == output.pools["matcher"].min_node_count * var.prices.machines["c4-highcpu-32"].on_demand
    )
    error_message = "The matcher pool's cost must be its node count at its machine's rate."
  }

  # Attribution splits the pools without inventing or losing money.
  assert {
    condition     = abs(sum(values(output.cost.by_service)) - output.cost.compute) < 1
    error_message = "Per-service attribution (${sum(values(output.cost.by_service))}) must reconcile with compute (${output.cost.compute})."
  }

  assert {
    condition = abs(
      output.cost.total - (output.cost.compute + output.cost.storage.total + output.cost.cluster_fee)
    ) < 1
    error_message = "The total must be compute plus storage plus the cluster fee."
  }

  # Only what exceeds the class baseline is billable, and the brokers are well
  # past it — so most of the file store's cost is performance, not capacity.
  assert {
    condition     = output.cost.storage.jetstream_performance > output.cost.storage.jetstream_capacity
    error_message = "Provisioned performance should dominate the file store's cost at this write rate."
  }

  # Committing buys nothing on disk or the cluster fee, so the saving is
  # bounded by the compute share and a three-year term cannot halve the bill
  # without also being most of it.
  assert {
    condition     = output.cost.compute / output.cost.total > 0.9
    error_message = "Compute should dominate; if it does not, the storage model has drifted."
  }
}

# A test environment is all floors and no throughput: one matcher per shard
# whatever the load, an availability floor on the brokers, and a fixed
# observability allowance. Under the dedicated layout each of those is a node
# of its own, so the deployment costs six nodes to run seven pods.
run "the_shared_layout_collapses_the_per_shape_node_floor" {
  command = plan

  variables {
    machines              = { shared = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 30 } }
    pool_layout           = "shared"
    observability_enabled = false

    shards                 = ["r1", "r3", "r6"]
    shard_precision        = 2
    binary_shard_precision = 2
    throughput_target_eps  = 1000
    design_target_eps      = 1000
    vertical_profile       = "small"
    largest_shard_file_mib = 413
    pricing_commitment     = "spot"

    nats_min_replicas           = 1
    nats_cpu_millis             = 1000
    nats_memory_mib             = 4096
    valkey_replicas_per_primary = 0
    valkey_cpu_millis           = 1000
    valkey_memory_mib           = 2048
    provisioned_iops            = 3000
    provisioned_throughput_mib  = 140
    collector_cpu_millis        = 500
    collector_memory_mib        = 1024
    matcher_working_set_mib     = 256
  }

  assert {
    condition     = length(keys(output.pools)) == 1
    error_message = "The shared layout must produce one pool, got ${join(", ", keys(output.pools))}."
  }

  # Three matchers, an orchestrator, a broker, a keyspace and a collector.
  assert {
    condition     = output.totals.pods == 7
    error_message = "Expected 7 pods, got ${output.totals.pods}."
  }

  assert {
    condition     = output.totals.nodes == 1
    error_message = "The whole test deployment should fit one node, got ${output.totals.nodes}."
  }

  # Tight but real: 6000m of requests against 7310m allocatable. If a profile
  # change pushes this over, the deployment silently needs a second node and
  # doubles.
  assert {
    condition     = output.pools["shared"].cpu_utilisation < 1
    error_message = "The shared pool is over-committed at ${output.pools["shared"].cpu_utilisation}."
  }

  # The budget this layout exists to hit. The cluster fee is nearly a third of
  # it, which is why a test environment wants a zonal cluster.
  assert {
    condition     = output.cost.total < 250
    error_message = "Expected the test tier under $250/month, got ${output.cost.total}."
  }

  assert {
    condition     = output.cost.cluster_fee > output.cost.storage.total
    error_message = "At this size the cluster fee outweighs all storage, which is worth knowing before optimising disks."
  }
}

# The brokers' spread floor has to follow them into whichever pool holds them,
# or the shared layout would quietly drop the guarantee.
run "the_broker_spread_floor_follows_the_layout" {
  command = plan

  variables {
    machines              = { shared = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 60 } }
    pool_layout           = "shared"
    observability_enabled = false

    shards                 = ["r1", "r3", "r6"]
    shard_precision        = 2
    binary_shard_precision = 2
    throughput_target_eps  = 1000
    design_target_eps      = 1000
    vertical_profile       = "small"
    largest_shard_file_mib = 413

    # Three brokers on one pool: the spread floor must still force three nodes,
    # even though every pod would otherwise fit on one.
    nats_min_replicas = 3
  }

  assert {
    condition     = output.nats.spread_nodes == 2
    error_message = "A 3-server cluster needs 2 nodes to survive a loss, got ${output.nats.spread_nodes}."
  }

  assert {
    condition     = output.totals.nodes >= output.nats.spread_nodes
    error_message = "The shared pool must still honour the broker spread floor."
  }
}

# Coverage is how a shard list generated over the wrong extent is caught before
# it becomes a fleet of idle matchers.
run "coverage_catches_shards_outside_the_service_region" {
  command = plan

  variables {
    shards                = ["r3gq", "9q8y"]
    throughput_target_eps = 10000
    coverage_cells        = ["r3", "r6"]
  }

  assert {
    condition     = join(",", output.out_of_coverage_shards) == "9q8y"
    error_message = "Expected 9q8y outside coverage, got ${join(",", output.out_of_coverage_shards)}."
  }
}
