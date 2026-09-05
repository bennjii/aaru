# The sizing model, recomputed here from the environment's shared inputs.
#
# This file is byte-identical in every unit, and `just validate` checks that it
# stays so. Each unit runs the model itself rather than reading another unit's
# state: it is pure arithmetic over the same inputs, so the fleet that was
# sized, the pools that were built and the topology that was deployed agree by
# construction, and no unit needs another's state to plan.

variable "sizing" {
  description = <<-EOT
    The capacity model's inputs, declared once per environment in
    `live/<env>/sizing.hcl` and passed to every unit unchanged. See
    `modules/capacity/variables.tf` for what each field means and why the
    wire-law fields (`streams`, and the precision behind `shards`) cannot
    change once events exist.
  EOT
  type = object({
    shards                = list(string)
    shard_precision       = number
    coverage_cells        = optional(list(string), [])
    throughput_target_eps = number
    design_target_eps     = number
    streams               = number
    vertical_profile      = string
    machines = map(object({
      machine_type = string
      vcpu         = number
      memory_gib   = number
    }))
  })
}

module "capacity" {
  source = "../../modules/capacity"

  shards                = var.sizing.shards
  shard_precision       = var.sizing.shard_precision
  coverage_cells        = var.sizing.coverage_cells
  throughput_target_eps = var.sizing.throughput_target_eps
  design_target_eps     = var.sizing.design_target_eps
  vertical_profile      = var.sizing.vertical_profile
  streams               = var.sizing.streams
  machines              = var.sizing.machines
}

check "capacity_is_deliverable" {
  # The model reports rather than fails, because a plan that stops at the
  # first problem hides the others. These surface as warnings on every plan;
  # read them before applying.
  assert {
    condition     = module.capacity.meets_target
    error_message = "The sizing model does not meet the target:\n${module.capacity.summary}"
  }

  assert {
    condition     = length(module.capacity.unschedulable_shapes) == 0
    error_message = "Pod shapes larger than their node: ${join(", ", module.capacity.unschedulable_shapes)}."
  }

  assert {
    condition     = length(module.capacity.out_of_coverage_shards) == 0
    error_message = "${length(module.capacity.out_of_coverage_shards)} shards fall outside coverage_cells, and each would become an idle matcher."
  }
}

output "capacity_summary" {
  description = "The sizing report. `just summary <env> <unit>`."
  value       = module.capacity.summary
}
