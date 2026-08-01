output "capacity_summary" {
  description = "The sizing report. `tofu output -raw capacity_summary`."
  value       = module.capacity.summary
}

output "cluster_name" {
  value = module.platform.cluster_name
}

output "image_registry" {
  description = "Pass to the workloads root as image_registry."
  value       = module.platform.image_registry
}

output "shard_bucket" {
  value = module.platform.shard_bucket
}

output "shard_cache_service_account_email" {
  value = module.platform.shard_cache_service_account_email
}

output "pool_node_selectors" {
  value = module.platform.pool_node_selectors
}

output "pool_tolerations" {
  value = module.platform.pool_tolerations
}

output "addressing" {
  description = "Pod range headroom. `pod_range_node_capacity` must stay above `max_nodes`."
  value       = module.platform.addressing
}
