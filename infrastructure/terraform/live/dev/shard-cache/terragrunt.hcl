# The shard cache bucket, its accounts and the public CDN copy for dev.
# Cluster-free; apply on its own to generate and publish shards.

include "root" {
  path   = find_in_parent_folders("root.hcl")
  expose = true
}

locals {
  env = include.root.locals.env
}

terraform {
  source = "${get_repo_root()}/infrastructure/terraform//units/shard-cache"
}

inputs = {
  env         = local.env.env
  bucket_name = local.env.shard_bucket
  labels      = local.env.labels

  cdn = merge(local.env.shard_cdn, {
    bucket_name = local.env.shard_public_bucket
  })
}
