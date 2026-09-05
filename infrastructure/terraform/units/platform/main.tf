# The cluster: network, GKE, node pools, and the bindings that tie the
# cluster-free units to it (node pull access on the registry, Workload Identity
# on the shard cache reader).
#
# Applied after registry and shard-cache, and before dependencies and realtime,
# whose kubernetes and helm providers configure themselves from the cluster this
# creates. Terragrunt orders the five through `dependency` blocks in
# live/<env>/*/terragrunt.hcl.

module "platform" {
  source = "../../modules/platform"

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
  node_pools = {
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
  }

  image_repository                  = var.image_repository
  shard_cache_service_account_id    = var.shard_cache_service_account_id
  workload_identity_namespace       = var.workload_namespace
  workload_identity_service_account = var.workload_service_account
  deletion_protection               = var.deletion_protection
  labels                            = var.labels
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
