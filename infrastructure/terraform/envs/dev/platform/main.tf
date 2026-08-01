# GCP substrate for the dev environment: network, cluster, node pools,
# registry and the shard bucket.
#
# Split from ../workloads because the kubernetes and helm providers there need
# a cluster endpoint to configure themselves, and a provider cannot be
# configured from a value that does not exist yet. Apply this root first.

locals {
  # One shard per line, as emitted by
  # `cargo run -p routers_shard --bin generate-shards`. Read from a file rather
  # than inlined: at precision 6 this list is thousands of entries.
  shards = compact(split("\n", trimspace(file(var.shards_file))))
}

module "capacity" {
  source = "../../../modules/capacity"

  shards                = local.shards
  shard_precision       = var.shard_precision
  cell_precision        = var.cell_precision
  coverage_cells        = var.coverage_cells
  throughput_target_eps = var.throughput_target_eps
  vertical_profile      = var.vertical_profile
  historian_queue_group = var.historian_queue_group
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

  shard_bucket_name   = var.shard_bucket_name
  deletion_protection = var.deletion_protection
  labels              = var.labels
}

# The sizing model reports these rather than failing on them, because a plan
# that stops at the first problem hides the others. A root should not apply
# past them.
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
