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

variable "shards_file" {
  description = "File holding one geohash shard per line, at shard_precision."
  type        = string
  default     = "shards.txt"
}

variable "shard_precision" {
  description = "Must match the length of every entry in shards_file."
  type        = number
  default     = 4
}

variable "cell_precision" {
  type    = number
  default = 2
}

variable "coverage_cells" {
  description = "Allowlist of cell prefixes for the service region. Catches a shard list generated over the wrong extent."
  type        = list(string)
  default     = []
}

variable "throughput_target_eps" {
  description = "A development figure. The six shipped shards sustain 36k evt/s at the standard profile, so anything above ~28k fails the capacity check."
  type        = number
  default     = 20000
}

variable "vertical_profile" {
  type    = string
  default = "standard"
}

variable "historian_queue_group" {
  description = "Whether historians share a cell's subject through a queue group, so replicas follow load."
  type        = bool
  default     = true
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
