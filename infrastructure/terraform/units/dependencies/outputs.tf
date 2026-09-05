# Read by the realtime unit through a Terragrunt `dependency` block. All three
# URLs are derived from fixed release names, so they are known at plan time.

output "nats_url" {
  value = module.nats.url
}

output "valkey_urls" {
  description = "The whole fleet. The realtime unit checks this against its own model, so a stale apply here is caught there."
  value       = module.valkey.urls
}

output "otlp_url" {
  value = module.observability.otlp_url
}

output "releases" {
  description = "Every Helm release across the three modules."
  value = concat(
    [module.nats.release_name],
    module.valkey.release_names,
    module.observability.release_names,
  )
}

output "totals" {
  description = "Dependency pod counts, for comparison against the capacity model."
  value = {
    nats_servers     = module.nats.replicas
    valkey_primaries = module.valkey.primaries
    valkey_pods      = module.valkey.pods
    collectors       = module.observability.collector_replicas
  }
}
