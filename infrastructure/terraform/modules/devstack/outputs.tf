# Service names are fixed with fullnameOverride, so these URLs are derived
# rather than read back from the releases. That keeps them known at plan time,
# which matters because the realtime module feeds them into Helm values.

locals {
  dns_suffix = "${var.namespace}.svc.cluster.local"
}

output "namespace" {
  description = "Namespace the dependencies were installed into."
  value       = var.namespace
}

output "nats_url" {
  description = "Client URL for the cluster. Every workload uses it, whatever cell its shard sits in."
  value       = "nats://nats.${local.dns_suffix}:4222"
}

output "valkey_urls" {
  description = <<-EOT
    The Valkey fleet. Every workload gets the whole list and places a vehicle by
    rendezvous hash over the URLs.

    Order carries no meaning: a primary's identity is its URL, so reordering
    this list moves nothing. Adding or removing one moves about 1/N of vehicles,
    which is the minimum any placement can achieve.
  EOT
  value = [
    for key in sort(keys(local.valkey_indices)) :
    "redis://valkey-${key}-primary.${local.dns_suffix}:6379"
  ]
}

output "valkey_primaries" {
  description = "Primaries in the fleet, and the modulus clients hash against."
  value       = var.valkey_primaries
}

output "otlp_url" {
  description = "OTLP http/protobuf endpoint. One collector deployment serves every cell."
  value       = "http://otel-collector-opentelemetry-collector.${local.dns_suffix}:4318"
}

output "valkey_client_mode" {
  description = "How clients are expected to address the fleet."
  value       = var.valkey_client_mode
}

output "valkey_client_prerequisite" {
  description = "Non-empty when the fleet is larger than the client can address."
  value = (var.valkey_client_mode == "single" && var.valkey_primaries > 1) ? join(" ", [
    "valkey_client_mode is 'single' with ${var.valkey_primaries} primaries, so",
    "only the first URL would be used and the rest would sit idle.",
  ]) : ""
}

output "release_names" {
  description = "Every Helm release this module manages."
  value = concat(
    [helm_release.nats.name],
    [for r in helm_release.valkey : r.name],
    [helm_release.collector.name],
    [for r in helm_release.prometheus : r.name],
  )
}

output "totals" {
  description = "Dependency pod counts, for comparison against the capacity model."
  value = {
    nats_servers     = var.nats_replicas
    valkey_primaries = var.valkey_primaries
    valkey_pods      = var.valkey_primaries * (1 + var.valkey_replicas_per_primary)
    collectors       = var.collector_replicas
  }
}
