# Service names are fixed with fullnameOverride, so the URLs are derived rather
# than read back from the releases. That keeps them known at plan time, which
# matters because the realtime unit feeds them into Helm values.

output "urls" {
  description = <<-EOT
    The fleet. Every workload gets the whole list and places a vehicle by
    rendezvous hash over the URLs.

    Order carries no meaning: a primary's identity is its URL, so reordering
    this list moves nothing. Adding or removing one moves about 1/N of vehicles,
    which is the minimum any placement can achieve.
  EOT
  value = [
    for key in sort(keys(local.valkey_indices)) :
    "redis://valkey-${key}-primary.${var.namespace}.svc.cluster.local:6379"
  ]
}

output "primaries" {
  description = "Primaries in the fleet, and the modulus clients hash against."
  value       = var.primaries
}

output "pods" {
  description = "Pods across the fleet, for comparison against the capacity model."
  value       = var.primaries * (1 + var.replicas_per_primary)
}

output "client_mode" {
  description = "How clients are expected to address the fleet."
  value       = var.client_mode
}

output "release_names" {
  value = [for r in helm_release.valkey : r.name]
}
