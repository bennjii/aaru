output "namespace" {
  description = "Namespace the workloads were installed into."
  value       = var.namespace
}

output "release_names" {
  description = "Every Helm release this module manages, cells first."
  value       = concat([for r in helm_release.cell : r.name], [helm_release.shared.name])
}

output "cell_count" {
  description = "Number of cell releases."
  value       = length(var.cell_plan)
}

output "shard_count" {
  description = "Shards deployed across all cells."
  value       = length(local.all_shards)
}

output "subjects" {
  description = <<-EOT
    The two-phase subjects in use, per shard. Cell and remaining geohash are
    separate tokens, so `<cell>.*` addresses a cell and `<cell>.<rest>` one
    shard. Each shard subject has exactly one subscriber, which is why matcher
    and orchestrator replicas are pinned at one.
  EOT
  value = {
    for shard in local.all_shards : shard => {
      cell     = substr(shard, 0, var.cell_precision)
      suffix   = substr(shard, var.cell_precision, var.shard_precision - var.cell_precision)
      position = "${var.subject_prefix}.position.${substr(shard, 0, var.cell_precision)}.${substr(shard, var.cell_precision, var.shard_precision - var.cell_precision)}"
      match    = "${var.subject_prefix}.match.${substr(shard, 0, var.cell_precision)}.${substr(shard, var.cell_precision, var.shard_precision - var.cell_precision)}"
      matched  = "${var.subject_prefix}.matched.${substr(shard, 0, var.cell_precision)}.${substr(shard, var.cell_precision, var.shard_precision - var.cell_precision)}"
    }
  }
}

output "redis_value" {
  description = "The REDIS env value rendered into the workloads: the fleet as a comma-separated list. Order carries no meaning; placement is a rendezvous hash over the URLs."
  value       = local.redis_value
}

output "expected_pod_count" {
  description = "Pods this module creates, for comparison against the capacity model's totals."
  value = (
    length(local.all_shards) * 2
    + (var.historian_mode == "per-shard" ? length(local.all_shards)
      : var.historian_mode == "per-cell" ? sum([for c in keys(var.cell_plan) : try(var.historian_replicas_by_cell[c], 1)])
    : 1)
  )
}
