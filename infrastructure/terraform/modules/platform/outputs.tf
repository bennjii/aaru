output "cluster_name" {
  value = google_container_cluster.cluster.name
}

output "cluster_endpoint" {
  description = "Control plane address, for the kubernetes and helm providers."
  value       = google_container_cluster.cluster.endpoint
  sensitive   = true
}

output "cluster_ca_certificate" {
  description = "Base64 cluster CA, for the kubernetes and helm providers."
  value       = google_container_cluster.cluster.master_auth[0].cluster_ca_certificate
  sensitive   = true
}

output "cluster_location" {
  value = google_container_cluster.cluster.location
}

output "network" {
  value = {
    vpc_id    = google_compute_network.vpc.id
    subnet_id = google_compute_subnetwork.nodes.id
  }
}

output "pool_names" {
  value = [for name, pool in google_container_node_pool.pool : pool.name]
}

output "pool_node_selectors" {
  description = "nodeSelector per pool, to pass to the realtime module's *_node_selector inputs."
  value       = local.pool_node_labels
}

output "pool_tolerations" {
  description = "Tolerations per pool, already in Kubernetes spelling."
  value       = local.pool_tolerations
}

output "node_service_account_email" {
  value = google_service_account.nodes.email
}

output "addressing" {
  description = "Pod range arithmetic, so a root can report how close the fleet is to exhausting it."
  value = {
    max_pods_per_node       = var.max_pods_per_node
    node_pod_slice_prefix   = local.node_pod_slice_prefix
    pod_range_node_capacity = local.pod_range_node_capacity
    max_nodes               = local.max_nodes
  }
}
