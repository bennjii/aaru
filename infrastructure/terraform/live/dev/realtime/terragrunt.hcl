# The routers-realtime release for dev. Apply last. This is the
# unit that changes on a rollout.

include "root" {
  path   = find_in_parent_folders("root.hcl")
  expose = true
}

locals {
  env    = include.root.locals.env
  sizing = read_terragrunt_config(find_in_parent_folders("sizing.hcl")).locals.sizing

  pools = ["matcher", "pipeline", "infra", "system"]

  # The image to roll out. Nothing is pinned yet: set ROUTERS_IMAGE_TAG for a
  # rollout, or replace the default here with the digest so the repository
  # records what is deployed and a rollback has something to return to.
  image_tag = get_env("ROUTERS_IMAGE_TAG", "")
}

terraform {
  source = "${get_repo_root()}/infrastructure/terraform//units/realtime"
}

dependency "registry" {
  config_path = "../registry"

  mock_outputs = {
    image_registry = "${local.env.region}-docker.pkg.dev/${local.env.project_id}/${local.env.image_repository}"
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

dependency "shard_cache" {
  config_path = "../shard-cache"

  mock_outputs = {
    shard_bucket = local.env.shard_bucket
    shard_cache_service_account = {
      id    = "projects/${local.env.project_id}/serviceAccounts/${local.env.env}-shard-cache@${local.env.project_id}.iam.gserviceaccount.com"
      email = "${local.env.env}-shard-cache@${local.env.project_id}.iam.gserviceaccount.com"
    }
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

dependency "platform" {
  config_path = "../platform"

  mock_outputs = {
    cluster_name        = "routers"
    pool_node_selectors = { for p in local.pools : p => { "routers.dev/pool" = p } }
    pool_tolerations = {
      for p in local.pools : p => [{
        key      = "routers.dev/pool"
        operator = "Equal"
        value    = p
        effect   = "NoSchedule"
      }]
    }
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

dependency "dependencies" {
  config_path = "../dependencies"

  mock_outputs = {
    nats_url = "nats://nats.${local.env.namespaces.dependencies}.svc.cluster.local:4222"
    otlp_url = "http://otel-collector-opentelemetry-collector.${local.env.namespaces.dependencies}.svc.cluster.local:4318"
    # Two primaries is what the dev sizing produces. The unit checks the count
    # against its own model, so a wrong mock shows up as that check firing.
    valkey_urls = [
      "redis://valkey-000-primary.${local.env.namespaces.dependencies}.svc.cluster.local:6379",
      "redis://valkey-001-primary.${local.env.namespaces.dependencies}.svc.cluster.local:6379",
    ]
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

inputs = {
  cluster_name = dependency.platform.outputs.cluster_name
  namespace    = local.env.namespaces.realtime

  # The chart lives in the repository, and the unit runs from a cache
  # directory, so the path has to be absolute.
  chart_path = "${get_repo_root()}/infrastructure/chart"

  image_registry                    = dependency.registry.outputs.image_registry
  shard_bucket                      = dependency.shard_cache.outputs.shard_bucket
  shard_cache_service_account_email = dependency.shard_cache.outputs.shard_cache_service_account.email
  workload_service_account          = local.env.workload_service_account
  pool_node_selectors               = dependency.platform.outputs.pool_node_selectors
  pool_tolerations                  = dependency.platform.outputs.pool_tolerations

  nats_url    = dependency.dependencies.outputs.nats_url
  valkey_urls = dependency.dependencies.outputs.valkey_urls
  otlp_url    = dependency.dependencies.outputs.otlp_url

  image_tag = local.image_tag

  sizing = local.sizing
}
