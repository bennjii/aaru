# Read by the dependencies and realtime units through Terragrunt `dependency`
# blocks, so nothing here is copied by hand. The registry prefix and the shard
# bucket come from their own units, not from here.

output "cluster_name" {
  value = module.platform.cluster_name
}

output "pool_node_selectors" {
  description = "nodeSelector per pool. The workloads take their placement from here, so a pool rename cannot strand them."
  value       = module.platform.pool_node_selectors
}

output "pool_tolerations" {
  description = "Tolerations per pool, already in Kubernetes spelling."
  value       = module.platform.pool_tolerations
}

output "addressing" {
  description = "Pod range headroom. `pod_range_node_capacity` must stay above `max_nodes`."
  value       = module.platform.addressing
}
