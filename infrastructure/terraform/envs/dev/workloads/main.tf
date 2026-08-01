# In-cluster half of the dev environment: NATS, Valkey, observability, and the
# routers-realtime releases.
#
# Applied after ../platform, which owns the cluster this points at. The sizing
# model is recomputed here rather than read from the other root's state — it is
# pure arithmetic over the same inputs, so the two agree by construction and
# neither root needs a backend to publish to the other.

locals {
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

module "devstack" {
  source = "../../../modules/devstack"

  namespace = var.dependency_namespace

  nats_replicas     = module.capacity.nats.replicas_total
  valkey_primaries  = module.capacity.valkey.primaries
  valkey_io_threads = module.capacity.valkey.io_threads

  collector_replicas = module.capacity.telemetry.collector_replicas

  # Dependencies share the infra pool; observability has its own.
  node_selector = { "routers.dev/pool" = "infra" }
  tolerations = [{
    key      = "routers.dev/pool"
    operator = "Equal"
    value    = "infra"
    effect   = "NoSchedule"
  }]

  observability_node_selector = { "routers.dev/pool" = "system" }
  observability_tolerations = [{
    key      = "routers.dev/pool"
    operator = "Equal"
    value    = "system"
    effect   = "NoSchedule"
  }]

  valkey_image = var.valkey_image
}

module "realtime" {
  source = "../../../modules/realtime"

  namespace  = var.namespace
  chart_path = "${path.module}/../../../../chart"

  cell_plan       = module.capacity.cell_plan
  shard_precision = var.shard_precision
  cell_precision  = var.cell_precision
  subject_prefix  = module.capacity.subject_prefix

  nats_url    = module.devstack.nats_url
  valkey_urls = module.devstack.valkey_urls
  otlp_url    = module.devstack.otlp_url

  profile                    = module.capacity.profile
  historian_mode             = module.capacity.historian_mode
  historian_queue_group      = var.historian_queue_group
  historian_replicas_by_cell = module.capacity.historian_by_cell

  telemetry_sample_ratio = tostring(module.capacity.telemetry.sample_ratio)

  image_registry    = var.image_registry
  image_tag         = var.image_tag
  image_pull_policy = var.image_pull_policy

  shard_cache_mode          = "gcsfuse"
  shard_cache_bucket        = var.shard_bucket
  service_account_create    = true
  service_account_name      = var.workload_service_account
  gcp_service_account_email = var.shard_cache_service_account_email

  matcher_node_selector  = { "routers.dev/pool" = "matcher" }
  pipeline_node_selector = { "routers.dev/pool" = "pipeline" }

  matcher_tolerations = [{
    key      = "routers.dev/pool"
    operator = "Equal"
    value    = "matcher"
    effect   = "NoSchedule"
  }]

  pipeline_tolerations = [{
    key      = "routers.dev/pool"
    operator = "Equal"
    value    = "pipeline"
    effect   = "NoSchedule"
  }]
}

check "capacity_is_deliverable" {
  assert {
    condition     = module.capacity.meets_target
    error_message = "The sizing model does not meet the target:\n${module.capacity.summary}"
  }
}
