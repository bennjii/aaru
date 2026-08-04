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

output "topology" {
  description = <<-EOT
    What was deployed on each axis. The two are independent: matchers divide
    geography, the fleet divides the vehicle partition space, and only the
    request subject connects them.
  EOT
  value = {
    shards       = module.realtime.shard_count
    matcher_pods = module.realtime.expected_pod_count.matchers
    matcher_max  = module.realtime.expected_pod_count.matchers_max
    fleet_size   = module.capacity.fleet.size
    partitions   = module.capacity.fleet.partitions
    per_pod      = module.capacity.fleet.partitions_per_pod
    raw_streams  = module.capacity.streams.raw
    file_store   = "${module.capacity.nats.file_store_gib}Gi per server"
  }
}

output "prerequisites" {
  description = "Non-empty entries are shortfalls the deployment cannot be configured out of. Read them before applying."
  value = {
    for name, message in {
      matched_stream = module.capacity.matched_stream_prerequisite
      raw_streams    = module.capacity.raw_stream_prerequisite
      precision      = module.capacity.precision_prerequisite
      valkey_client  = module.capacity.valkey_client_prerequisite
    } : name => message if message != ""
  }
}
