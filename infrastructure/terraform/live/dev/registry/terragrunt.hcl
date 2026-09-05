# Artifact Registry for dev. Cluster-free; apply on its own.

include "root" {
  path   = find_in_parent_folders("root.hcl")
  expose = true
}

locals {
  env = include.root.locals.env
}

terraform {
  source = "${get_repo_root()}/infrastructure/terraform//units/registry"
}

inputs = {
  env           = local.env.env
  repository_id = local.env.image_repository
  labels        = local.env.labels
}
