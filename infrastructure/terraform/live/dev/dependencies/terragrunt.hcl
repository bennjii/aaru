# NATS, Valkey and observability for dev. Apply after platform.

include "root" {
  path   = find_in_parent_folders("root.hcl")
  expose = true
}

locals {
  env    = include.root.locals.env
  sizing = read_terragrunt_config(find_in_parent_folders("sizing.hcl")).locals.sizing

  pools = ["matcher", "pipeline", "infra", "system"]
}

terraform {
  source = "${get_repo_root()}/infrastructure/terraform//units/dependencies"
}

dependency "platform" {
  config_path = "../platform"

  # Stand-ins with the real shape, so this unit plans and validates before the
  # platform has been applied. Never used for apply.
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

inputs = {
  cluster_name = dependency.platform.outputs.cluster_name
  namespace    = local.env.namespaces.dependencies

  pool_node_selectors = dependency.platform.outputs.pool_node_selectors
  pool_tolerations    = dependency.platform.outputs.pool_tolerations

  valkey_image = local.env.valkey_image

  # Per-volume provisioning for the JetStream file store. Above the model's
  # derived demand, not equal to it: the derived figure is a mean. The unit
  # fails the plan if these drop below what the model needs, and if they exceed
  # what the infra pool's machine type can deliver.
  jetstream_provisioned_throughput_mib = 750
  jetstream_provisioned_iops           = 30000

  # c4-standard-16, the infra pool's machine type in sizing.hcl.
  jetstream_instance_iops_limit           = 100000
  jetstream_instance_throughput_limit_mib = 1600

  sizing = local.sizing
}
