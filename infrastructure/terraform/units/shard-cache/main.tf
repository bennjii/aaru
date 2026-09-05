# The shard cache and, when enabled, its public CDN copy. Cluster-free: apply
# this on its own to generate and publish shards without a cluster running.
#
# Consumers: the platform unit takes the reader account to bind Workload
# Identity to it, and the realtime unit takes the bucket and the reader's email
# for the gcsfuse mount. Browsers take `cdn.base_url`.

module "shard_cache" {
  source = "../../modules/shard-cache"

  project_id          = var.project_id
  region              = var.region
  env                 = var.env
  bucket_name         = var.bucket_name
  keep_versions       = var.keep_versions
  deletion_protection = var.deletion_protection
  labels              = var.labels
}

module "cdn" {
  source = "../../modules/shard-cdn"
  count  = var.cdn.enabled ? 1 : 0

  project_id = var.project_id
  region     = var.region
  env        = var.env
  name       = "${var.env}-routers"

  bucket_name                     = var.cdn.bucket_name
  publisher_service_account_email = module.shard_cache.publisher_service_account_email
  hostname                        = var.cdn.hostname
  cors_origins                    = var.cdn.cors_origins
  cache_ttl_seconds               = var.cdn.cache_ttl_seconds

  keep_versions       = var.keep_versions
  deletion_protection = var.deletion_protection
  labels              = var.labels
}

check "the_public_bucket_is_not_the_private_one" {
  assert {
    condition     = !var.cdn.enabled || var.cdn.bucket_name != var.bucket_name
    error_message = "cdn.bucket_name equals bucket_name. The matchers' bucket enforces public access prevention; the CDN needs a second, public bucket."
  }
}
