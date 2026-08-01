output "shard_count" {
  description = "Shards deployed; equals the matcher and orchestrator totals."
  value       = local.shard_count
}

output "shard_precision" {
  description = "Geohash precision, passed through to the matcher's PRECISION."
  value       = var.shard_precision
}

output "profile" {
  description = "Resolved vertical profile: per-pod resources and per-shard throughput."
  value       = local.profile
}

output "capacity_eps" {
  description = "Events per second this shard count sustains at this profile."
  value       = local.capacity_eps
}

output "required_eps" {
  description = "Target plus headroom; what capacity_eps must reach."
  value       = local.required_eps
}

output "meets_target" {
  description = "Whether shards, profile and Valkey topology all reach the target. Env roots assert on this."
  value       = local.meets_target
}

output "shards_required" {
  description = "Shards needed at this profile to reach the target with headroom."
  value       = local.shards_required
}

output "shard_deficit" {
  description = "Shards missing; zero when met."
  value       = local.shard_deficit
}

output "recommended_precision" {
  description = "Precision that would close the shard deficit. Regenerate the shard cache at this precision before adopting it."
  value       = local.recommended_precision
}

output "replicas" {
  description = "Pod counts per service. Matcher and orchestrator are pinned at one per shard."
  value = {
    matcher      = local.matcher_pods
    orchestrator = local.orchestrator_pods
    historian    = local.historian_pods
  }
}

output "historian_mode" {
  description = "How historian work is partitioned: per cell, per shard, or globally on <prefix>.position.>."
  value       = var.historian_mode
}

# --- Cells and groups -------------------------------------------------------

output "cells" {
  description = "Cell prefixes in use, derived from the shard list. One historian subscription and one Helm release each."
  value       = local.cells
}

output "cell_precision" {
  description = "Geohash prefix length defining a cell, and the first geohash token of every subject."
  value       = var.cell_precision
}

output "cell_plan" {
  description = "Per cell: its shards and its event rate. Drives the realtime module's Helm releases."
  value       = local.cell_plan
}

output "shards_by_cell" {
  description = "Shard list per cell. Each cell becomes one Helm release."
  value       = local.shards_by_cell
}

output "subjects" {
  description = <<-EOT
    Per shard, the two-phase subjects it publishes and subscribes to. The cell
    and the remaining geohash are separate tokens, so a wildcard can address
    either a whole cell or one shard.
  EOT
  value       = local.subjects
}

output "historian_subjects" {
  description = "What each historian subscribes to, keyed by cell, shard or \"global\" depending on historian_mode."
  value       = local.historian_subjects
}

output "subject_prefix" {
  description = "Root of the subject scheme."
  value       = var.subject_prefix
}

output "out_of_coverage_shards" {
  description = "Shards not nesting under any `coverage_cells` entry. Must be empty when coverage is declared."
  value       = local.out_of_coverage
}

# --- Dependencies -----------------------------------------------------------

output "nats" {
  description = <<-EOT
    The single core NATS cluster. Sized from the total delivery rate, and
    independent of cell layout: a server forwards a message only to routes with
    matching subscription interest, so clustering already partitions by subject.

    `route_connections` is the full mesh, N(N-1)/2. It is the number to watch
    before growing further; gateways and a supercluster are the documented
    answer once it stops being reasonable.
  EOT
  value = {
    replicas_total    = local.nats_replicas_total
    delivery_rate     = local.nats_delivery_rate
    msgs_per_server   = var.nats_msgs_per_server
    route_connections = local.nats_route_connections
    cpu_millis        = var.nats_cpu_millis
    memory_mib        = var.nats_memory_mib
  }
}

output "valkey" {
  description = <<-EOT
    The Valkey fleet. One logical keyspace for the whole deployment, sharded by
    `hash(vehicle_id)` rather than by geography, so no boundary affects history
    continuity.
  EOT
  value = {
    client_mode          = var.valkey_client_mode
    primaries            = local.valkey_primaries_total
    pods                 = local.valkey_pods_total
    replicas_per_primary = var.valkey_replicas_per_primary
    ops_rate             = local.valkey_ops_rate
    ops_per_primary      = var.valkey_ops_per_primary
    io_threads           = var.valkey_io_threads
    cpu_millis           = var.valkey_cpu_millis
    memory_mib           = var.valkey_memory_mib
  }
}

output "single_valkey_ceiling_eps" {
  description = "Event rate one primary serves, at valkey_ops_per_event commands each. Also the whole deployment's ceiling under `valkey_client_mode = \"single\"`."
  value       = local.single_valkey_ceiling_eps
}

output "valkey_client_prerequisite" {
  description = <<-EOT
    Non-empty when the fleet is larger than the client can address. Adding
    primaries does not help: the client must be able to reach them.
  EOT
  value = local.valkey_client_is_sufficient ? "" : join(" ", [
    "valkey_client_mode is 'single' but the target needs ${local.valkey_primaries_total} primaries.",
    "Clients would reach one primary and the deployment would cap at",
    "${local.single_valkey_ceiling_eps} evt/s.",
    "Switch to 'pooled-hash': RedisStore already spreads vehicles across the",
    "whole fleet by rendezvous hash.",
  ])
}

output "historian_saturated" {
  description = "True when any historian is asked to absorb more than `historian_eps_per_pod`."
  value       = local.historian_is_saturated
}

output "saturated_historian_cells" {
  description = <<-EOT
    Cells whose historian cannot keep up. Fix by enabling
    `historian_queue_group` so replicas share the subject, by raising
    `cell_precision`, or by moving to `per-shard`. The queue group is usually
    right: it leaves cells coarse, so the fleet gains historians without also
    gaining a Helm release per cell.
  EOT
  value       = local.saturated_historians
}

output "historian_by_cell" {
  description = "Historian replicas per cell. Above one only with a queue group, which needs the historian.rs change."
  value       = local.historian_by_cell
}

output "historian_queue_group" {
  description = "Whether historians share their subject through a queue group."
  value       = var.historian_queue_group
}

output "historian_eps_per_pod" {
  description = "Mean event rate per historian, for comparison against the per-pod ceiling."
  value       = local.historian_eps_per_pod
}

output "telemetry" {
  description = "Collector sizing and the sample ratio the services must apply."
  value = {
    sample_ratio       = var.telemetry_sample_ratio
    span_rate          = local.span_rate
    collector_replicas = local.collector_replicas
    cpu_millis         = var.collector_cpu_millis
    memory_mib         = var.collector_memory_mib
  }
}

# --- Nodes ------------------------------------------------------------------

output "pools" {
  description = "Node pool sizing per pool: machine type, autoscaler floor and ceiling, allocatable capacity, and the demand behind it."
  value       = local.pools
}

output "unschedulable_shapes" {
  description = "Pod shapes larger than one node's allocatable capacity, as \"<pool>/<shape>\". Must be empty."
  value       = local.unschedulable_shapes
}

output "totals" {
  description = "Fleet totals, for cost estimation and for sizing the pod CIDR."
  value = {
    nodes            = local.total_nodes
    vcpu             = local.total_vcpu
    pods             = local.total_pods
    pod_ips_required = local.pod_ips_required
  }
}

output "summary" {
  description = "Human-readable sizing report. `tofu output -raw capacity_summary` in an env root."
  value       = <<-EOT
    target        ${format("%d", var.throughput_target_eps)} evt/s (+${format("%d", var.headroom_ratio * 100)}% headroom = ${format("%d", local.required_eps)})
    profile       ${var.vertical_profile} (${local.profile.shard_eps} evt/s per shard, ${local.profile.matcher_workers} matcher / ${local.profile.orchestrator_workers} orchestrator workers)
    shards        ${local.shard_count} at precision ${var.shard_precision}
    cells         ${local.cell_count} at precision ${var.cell_precision} (historian subject, helm release)
    subjects      ${var.subject_prefix}.position.<cell>.<rest>
    capacity      ${format("%d", local.capacity_eps)} evt/s
    verdict       ${local.meets_target ? "MEETS TARGET" : "SHORT"}
    ${local.shard_deficit > 0 ? format("shards        short by %d; try precision %d", local.shard_deficit, local.recommended_precision) : "shards        sufficient"}
    ${local.valkey_client_is_sufficient ? "valkey        sufficient" : format("valkey        client mode 'single' caps this at %d evt/s; needs pooled-hash for %d primaries", local.single_valkey_ceiling_eps, local.valkey_primaries_total)}
    ${local.historian_is_saturated ? format("historian     %d cell(s) exceed the %d evt/s one pod sustains; enable historian_queue_group, or raise cell_precision, or use per-shard", length(local.saturated_historians), var.historian_eps_per_pod) : "historian     sufficient"}

    pods          ${local.matcher_pods} matcher, ${local.orchestrator_pods} orchestrator, ${local.historian_pods} historian (${var.historian_mode})
    nats          ${local.nats_replicas_total} servers, one cluster (${format("%d", local.nats_delivery_rate)} deliveries/s, ${local.nats_route_connections} routes)
    valkey        ${local.valkey_primaries_total} primaries + ${local.valkey_primaries_total * var.valkey_replicas_per_primary} replicas, ${var.valkey_client_mode}, ${var.valkey_io_threads} io-threads
    collectors    ${local.collector_replicas} at ${var.telemetry_sample_ratio} sample ratio (${format("%d", local.span_rate)} spans/s)

    nodes         ${local.total_nodes} total, ${local.total_vcpu} vCPU, ${local.pod_ips_required} pod IPs
    ${join("\n    ", [for p, v in local.pools : format("%-14s%2d-%2d x %-20s %4d pods, cpu %d%%", p, v.min_node_count, v.max_node_count, v.machine_type, v.demand.pods, floor(v.cpu_utilisation * 100))])}
  EOT
}
