output "namespace" {
  description = "Namespace the workloads were installed into."
  value       = var.namespace
}

output "release_names" {
  description = "Every Helm release this module manages. One, deliberately: the orchestrator fleet must be rendered exactly once."
  value       = [helm_release.realtime.name]
}

output "shard_count" {
  description = "Shards deployed, and so matcher Deployments."
  value       = length(var.shards)
}

output "subjects" {
  description = <<-EOT
    The subjects in use.

    A shard is one token now, not two: the orchestrator routes a solve to
    `events.match.<shard_of(point)>`, so nothing needs to address a group of
    shards with a wildcard. Raw and matched events are keyed by vehicle
    partition instead, which is why their subjects carry a number rather than a
    geohash.
  EOT
  value = {
    match   = { for shard in var.shards : shard => "events.match.${shard}" }
    raw     = "events.raw.p.<partition>"
    matched = "events.matched.p.<partition>"
  }
}

output "redis_value" {
  description = "The REDIS env value rendered into the workloads: the fleet as a comma-separated list. Order carries no meaning; placement is a rendezvous hash over the URLs."
  value       = local.redis_value
}

output "expected_pod_count" {
  description = <<-EOT
    Pods this module creates at the HPA floor, for comparison against the
    capacity model's totals. The matcher half can grow to
    `shards * matcher_replicas_max`; the fleet cannot grow at all without
    re-slicing the partition space.
  EOT
  value = {
    matchers      = length(var.shards) * var.matcher_replicas
    matchers_max  = length(var.shards) * var.matcher_replicas_max
    orchestrators = var.fleet_size
    total         = length(var.shards) * var.matcher_replicas + var.fleet_size
  }
}
