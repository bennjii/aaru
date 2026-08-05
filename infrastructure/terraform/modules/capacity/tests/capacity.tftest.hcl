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
    infra    = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 64 }
    system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
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

  # 1M / 8000 per pod = 125, snapped up to 128 so every pod takes exactly 8
  # partitions. The fleet is independent of the shard count entirely.
  assert {
    condition     = output.fleet.size == 128
    error_message = "Expected a fleet of 128, got ${output.fleet.size}."
  }

  assert {
    condition     = output.fleet.partitions_per_pod == 8
    error_message = "Expected 8 partitions per pod, got ${output.fleet.partitions_per_pod}."
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
