variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "australia-southeast1"
}

variable "env" {
  type    = string
  default = "dev"
}

variable "cluster_name" {
  type    = string
  default = "routers"
}

variable "network_name" {
  type    = string
  default = "routers"
}

variable "subnet_cidr" {
  type    = string
  default = "10.0.0.0/20"
}

variable "pods_secondary_cidr" {
  description = "Sized for the node ceiling, not the steady state. At 110 pods per node a /14 holds 1024 nodes; the platform module fails the plan if the pools can outgrow it."
  type        = string
  default     = "10.16.0.0/14"
}

variable "services_secondary_cidr" {
  type    = string
  default = "10.32.0.0/20"
}

variable "master_authorized_cidrs" {
  description = "Networks allowed to reach the control plane. Empty closes it to everything outside Google Cloud, including CI."
  type = list(object({
    cidr_block   = string
    display_name = string
  }))
  default = []
}

variable "shard_bucket_name" {
  type = string
}

# --- Sizing -----------------------------------------------------------------

variable "shards_file" {
  description = "File holding one geohash shard per line, at shard_precision."
  type        = string
  default     = "shards.txt"
}

variable "shard_precision" {
  description = "Must match the length of every entry in shards_file, and the `event::SHARD_PRECISION` compiled into the binaries."
  type        = number
  default     = 4
}

variable "coverage_cells" {
  description = "Allowlist of geohash prefixes for the service region. Catches a shard list generated over the wrong extent before it becomes a fleet of idle matchers."
  type        = list(string)
  default     = []
}

variable "throughput_target_eps" {
  description = <<-EOT
    The rate to size for. Set to production's current load rather than a
    development figure, so the plan is honest about node counts and cost.

    Note what it does against the six shipped shards: 800k + headroom over six
    geographic subjects is ~167k evt/s each, which the model covers with ~28
    matcher replicas per shard. That is legitimate — a shard's replicas are a
    NATS queue group, so they divide its requests — but every replica loads that
    shard's whole `.shard.rt` file, so the memory cost is linear in replicas.
    Regenerating `shards.txt` over the real coverage at a finer precision trades
    that duplication for more subjects, and is the better shape past a point;
    `max_matcher_replicas_per_shard` in the capacity module is where that point
    is expressed.
  EOT
  type        = number
  default     = 800000
}

variable "design_target_eps" {
  description = "The rate the deployment is designed to reach. Only the wire-law quantities — `streams` and the partition count — must satisfy it today, because both are a migration to change and everything else is a variable."
  type        = number
  default     = 5000000
}

variable "streams" {
  description = <<-EOT
    Raw JetStream streams the vehicle partition space divides across. Pinned for
    the life of the deployment: a revision is a stream sequence, so remapping
    partitions resets sequence domains.

    64 puts the design target at ~98k writes/s per stream, inside a single raft
    leader's ceiling. 32 would put it at ~195k, over it.
  EOT
  type        = number
  default     = 64
}

variable "vertical_profile" {
  type    = string
  default = "standard"
}

variable "machines" {
  description = "Machine shape per node pool. Keys must match the capacity module's pools: matcher, pipeline, infra, system."
  type = map(object({
    machine_type = string
    vcpu         = number
    memory_gib   = number
  }))

  default = {
    matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
    pipeline = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 64 }
    infra    = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
    system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 32 }
  }
}

variable "deletion_protection" {
  type    = bool
  default = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
