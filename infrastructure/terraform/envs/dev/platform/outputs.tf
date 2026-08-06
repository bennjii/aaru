output "capacity_summary" {
  description = "The sizing report. `tofu output -raw capacity_summary`."
  value       = module.capacity.summary
}

output "cost_report" {
  description = "Estimated monthly spend for the sized fleet. `just cost`, or `just cost-what-if 'pricing_commitment=cud_3y'` to compare tiers without applying."
  value       = module.capacity.cost_report
}

output "cost" {
  description = "The same estimate as structured data, broken down by node pool and by service."
  value       = module.capacity.cost
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
