# The capacity model's inputs for dev, declared once.
#
# Every unit under this directory receives this object unchanged and runs the
# model over it (units/*/sizing.tf), so the node pools that were built, the
# brokers that were installed and the fleet that was deployed cannot disagree
# about the target. Change a value here and every unit's plan moves together.
#
# Two fields are wire law and cannot change once events exist: `streams`, and
# the precision the shards are cut at. Everything else is a variable change.

locals {
  sizing = {
    # One shard per line, as emitted by
    # `cargo run -p routers_shard --bin generate-shards`. A file rather than a
    # list: at a finer precision this is thousands of entries.
    shards = compact(split("\n", trimspace(file("${get_terragrunt_dir()}/shards.txt"))))

    # Must match the length of every entry in shards.txt, and the
    # `event::SHARD_PRECISION` compiled into the binaries. The model checks
    # both and reports a disagreement as a prerequisite on every plan.
    shard_precision = 4

    # Allowlist of geohash prefixes for the service region. Catches a shard
    # list generated over the wrong extent before it becomes a fleet of idle
    # matchers. Empty disables the check.
    coverage_cells = []

    # The rate to size for. Production's current load rather than a
    # development figure, so the plan is honest about node counts and cost.
    #
    # Against the six shipped shards this is ~167k evt/s each, which the model
    # covers with ~28 matcher replicas per shard. Legitimate — a shard's
    # replicas are a NATS queue group — but every replica loads that shard's
    # whole `.shard.rt` file, so memory is linear in replicas. Regenerating
    # shards.txt at a finer precision trades that duplication for more
    # subjects, and is the better shape past a point.
    throughput_target_eps = 800000

    # The rate the deployment is designed to reach. Only the wire-law
    # quantities — `streams` and the partition count — must satisfy it today,
    # because both are a migration to change.
    design_target_eps = 5000000

    # Raw JetStream streams the vehicle partition space divides across. Pinned
    # for the life of the deployment: a revision is a stream sequence, so
    # remapping partitions resets sequence domains and breaks every revision
    # comparison across the boundary.
    #
    # 64 puts the design target at ~98k writes/s per stream, inside a single
    # raft leader's ceiling. 32 would put it at ~195k, over it.
    streams = 64

    vertical_profile = "standard"

    # Machine shape per node pool. `vcpu` and `memory_gib` must match
    # `machine_type`: the model derives allocatable capacity from them, so an
    # overstated figure plans too few nodes and the shortfall appears as
    # Pending pods rather than as a wrong number here.
    #
    # Every pool is CPU-bound at these pod shapes, which is why the two
    # carrying the pipeline are highcpu. The infra pool is deliberately not
    # packed tight: the model floors its node count so the brokers can spread
    # across failure domains. If its machine type changes, change
    # `jetstream_instance_*_limit` in dependencies/terragrunt.hcl with it.
    machines = {
      matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
      pipeline = { machine_type = "c4-highcpu-16", vcpu = 16, memory_gib = 32 }
      infra    = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 60 }
      system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 30 }
    }
  }
}
