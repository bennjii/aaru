output "shard_bucket" {
  description = "For the realtime unit's gcsfuse mount."
  value       = module.shard_cache.bucket
}

output "shard_cache_service_account" {
  description = "`id` for the platform unit's Workload Identity binding, `email` for the realtime unit's service account annotation."
  value       = module.shard_cache.reader_service_account
}

output "publisher_service_account_email" {
  description = "Run `generate-shards` uploads as this."
  value       = module.shard_cache.publisher_service_account_email
}

output "cdn" {
  description = "Null when the CDN is off. Otherwise the public bucket, the global address to point DNS at, and the URL browsers fetch shards from."
  value = var.cdn.enabled ? {
    bucket   = module.cdn[0].bucket
    address  = module.cdn[0].address
    base_url = module.cdn[0].base_url
  } : null
}
