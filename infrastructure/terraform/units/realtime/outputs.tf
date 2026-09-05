output "releases" {
  value = module.realtime.release_names
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
