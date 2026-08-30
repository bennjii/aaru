# GCP substrate for the dev environment: network, cluster, node pools,
# registry and the shard bucket.
#
# Split from ../workloads because the kubernetes and helm providers there need
# a cluster endpoint to configure themselves, and a provider cannot be
# configured from a value that does not exist yet. Apply this root first.

locals {
  # One shard per line, as emitted by
  # `cargo run -p routers_shard --bin generate-shards`. Read from a file rather
  # than inlined: at a finer precision this list is thousands of entries.
  shards = compact(split("\n", trimspace(file(var.shards_file))))
}

module "capacity" {
  source = "../../../modules/capacity"

  shards                = local.shards
  shard_precision       = var.shard_precision
  coverage_cells        = var.coverage_cells
  throughput_target_eps = var.throughput_target_eps
  design_target_eps     = var.design_target_eps
  vertical_profile      = var.vertical_profile
  streams               = var.streams
  machines              = var.machines
}

module "platform" {
  source = "../../../modules/platform"

  project_id   = var.project_id
  region       = var.region
  env          = var.env
  cluster_name = var.cluster_name
  network_name = var.network_name

  subnet_cidr             = var.subnet_cidr
  pods_secondary_cidr     = var.pods_secondary_cidr
  services_secondary_cidr = var.services_secondary_cidr
  master_authorized_cidrs = var.master_authorized_cidrs

  # Straight from the sizing model, so the fleet that gets built is the fleet
  # that was sized. Taints keep each pool for its own workload.
  #
  # Gated on workers_enabled: false hands the platform module an empty map, so
  # every google_container_node_pool is destroyed and only the cluster and its
  # surrounding infra remain. Flipping back to true recreates the pools; nothing
  # else in this root has a diff.
  node_pools = var.workers_enabled ? {
    for name, pool in module.capacity.pools : name => {
      machine_type   = pool.machine_type
      min_node_count = pool.min_node_count
      max_node_count = pool.max_node_count
      taints = [{
        key    = "routers.dev/pool"
        value  = name
        effect = "NO_SCHEDULE"
      }]
    }
  } : {}

  shard_bucket_name   = var.shard_bucket_name
  deletion_protection = var.deletion_protection
  labels              = var.labels
}

# The sizing model reports rather than fails, because a plan that stops at the
# first problem hides the others. These surface as warnings on every plan; read
# them before applying.
check "capacity_is_deliverable" {
  assert {
    condition     = module.capacity.meets_target
    error_message = "The sizing model does not meet the target:\n${module.capacity.summary}"
  }

  assert {
    condition     = length(module.capacity.unschedulable_shapes) == 0
    error_message = "Pod shapes larger than their node: ${join(", ", module.capacity.unschedulable_shapes)}."
  }

  assert {
    condition     = length(module.capacity.out_of_coverage_shards) == 0
    error_message = "${length(module.capacity.out_of_coverage_shards)} shards fall outside coverage_cells, and each would become an idle matcher."
  }
}

# The two quantities that cannot be corrected once events exist. Everything
# else in the model is a variable change; these are a migration, so they are
# checked against the design target rather than today's.
check "the_wire_law_survives_the_design_target" {
  assert {
    condition = module.capacity.design_target.streams_ok
    error_message = join(" ", [
      "The pinned stream count cannot absorb the design target:",
      "${module.capacity.design_target.writes_per_stream} writes/s per stream",
      "against a ${module.capacity.streams.raw}-stream topology.",
      "A revision is a stream sequence, so raising this later resets sequence",
      "domains and breaks every revision comparison across the boundary.",
      "Raise `streams` to ${module.capacity.design_target.streams_required} now, while the streams are still empty.",
    ])
  }

  assert {
    condition     = module.capacity.design_target.fleet_fits
    error_message = "The design target needs a fleet of ${module.capacity.design_target.fleet_required} against ${module.capacity.fleet.partitions} partitions. A partition is the smallest unit an owner can hold, so the answer is a larger vertical profile, not more pods."
  }
}

check "the_binaries_agree_on_precision" {
  assert {
    condition     = module.capacity.precision_prerequisite == ""
    error_message = module.capacity.precision_prerequisite
  }
}
