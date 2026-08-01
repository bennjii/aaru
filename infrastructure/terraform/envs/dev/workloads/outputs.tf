output "capacity_summary" {
  description = "The sizing report. `tofu output -raw capacity_summary`."
  value       = module.capacity.summary
}

output "nats_url" {
  value = module.devstack.nats_url
}

output "valkey_urls" {
  value = module.devstack.valkey_urls
}

output "releases" {
  description = "Every Helm release across both modules."
  value       = concat(module.devstack.release_names, module.realtime.release_names)
}

output "dependency_totals" {
  value = module.devstack.totals
}
