# The cluster for dev. Apply after registry and shard-cache.

include "root" {
  path   = find_in_parent_folders("root.hcl")
  expose = true
}

locals {
  env    = include.root.locals.env
  sizing = read_terragrunt_config(find_in_parent_folders("sizing.hcl")).locals.sizing
}

terraform {
  # `//` marks where the copied tree ends and the unit begins: the whole of
  # infrastructure/terraform is copied into .terragrunt-cache, so the unit's
  # ../../modules references still resolve there.
  source = "${get_repo_root()}/infrastructure/terraform//units/platform"
}

dependency "registry" {
  config_path = "../registry"

  mock_outputs = {
    repository = { location = local.env.region, name = local.env.image_repository }
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

dependency "shard_cache" {
  config_path = "../shard-cache"

  mock_outputs = {
    shard_cache_service_account = {
      id    = "projects/${local.env.project_id}/serviceAccounts/${local.env.env}-shard-cache@${local.env.project_id}.iam.gserviceaccount.com"
      email = "${local.env.env}-shard-cache@${local.env.project_id}.iam.gserviceaccount.com"
    }
  }
  mock_outputs_allowed_terraform_commands = ["init", "validate", "plan", "show"]
}

inputs = {
  env                      = local.env.env
  cluster_name             = local.env.cluster_name
  workload_namespace       = local.env.namespaces.realtime
  workload_service_account = local.env.workload_service_account
  labels                   = local.env.labels

  image_repository               = dependency.registry.outputs.repository
  shard_cache_service_account_id = dependency.shard_cache.outputs.shard_cache_service_account.id

  sizing = local.sizing
}
