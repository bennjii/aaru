# The routers-realtime release: matchers and the orchestrator fleet.
#
# Applied last. The cluster comes from the platform unit and the NATS, Valkey
# and OTLP URLs from the dependencies unit, both through Terragrunt
# `dependency` blocks in live/<env>/realtime/terragrunt.hcl. This is the unit
# that changes on every rollout, which is why it holds its own state.

module "realtime" {
  source = "../../modules/realtime"

  namespace  = var.namespace
  chart_path = var.chart_path

  shards          = var.sizing.shards
  shard_precision = var.sizing.shard_precision

  # Both from the sizing model, so the deployed topology and the model that
  # justified it cannot drift. `streams` is also what the orchestrator creates
  # its streams with, so every unit takes it from the same sizing.hcl.
  streams    = var.sizing.streams
  fleet_size = module.capacity.fleet.size

  matcher_replicas     = module.capacity.matcher.replicas
  matcher_memory_mib   = module.capacity.matcher.memory_mib
  matcher_replicas_max = module.capacity.matcher.replicas_max

  nats_url    = var.nats_url
  valkey_urls = var.valkey_urls
  otlp_url    = var.otlp_url

  profile = module.capacity.profile

  telemetry_sample_ratio = tostring(module.capacity.telemetry.sample_ratio)

  image_registry    = var.image_registry
  image_tag         = var.image_tag
  image_pull_policy = var.image_pull_policy

  shard_cache_mode          = "gcsfuse"
  shard_cache_bucket        = var.shard_bucket
  service_account_create    = true
  service_account_name      = var.workload_service_account
  gcp_service_account_email = var.shard_cache_service_account_email

  # Placement comes from the platform unit's outputs, so a pool's taint and its
  # workloads' tolerations cannot drift apart.
  matcher_node_selector  = var.pool_node_selectors["matcher"]
  pipeline_node_selector = var.pool_node_selectors["pipeline"]
  matcher_tolerations    = var.pool_tolerations["matcher"]
  pipeline_tolerations   = var.pool_tolerations["pipeline"]
}

# The one shortfall the current binaries cannot be configured out of. Reported
# here as well as in the model, because this is the unit that deploys the thing
# doing the publishing.
check "the_matched_stream_can_absorb_the_emissions" {
  assert {
    condition     = module.capacity.streams.matched_sufficient
    error_message = module.capacity.matched_stream_prerequisite
  }
}

# The dependencies unit and this one run the same model over the same
# sizing.hcl, so they agree by construction — unless one was applied against an
# older sizing than the other. The fleet the workloads address is the one place
# that shows up before traffic does.
check "the_dependencies_were_sized_from_the_same_model" {
  assert {
    condition     = length(var.valkey_urls) == module.capacity.valkey.primaries
    error_message = "The dependencies unit built ${length(var.valkey_urls)} Valkey primaries but this unit's model expects ${module.capacity.valkey.primaries}. Both read live/<env>/sizing.hcl, so one was applied against a stale sizing; re-apply dependencies first."
  }
}
