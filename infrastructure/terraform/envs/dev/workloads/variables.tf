variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "australia-southeast1"
}

variable "cluster_name" {
  description = "Cluster from the ../platform root. Looked up by data source, so that root must be applied first."
  type        = string
  default     = "routers"
}

variable "namespace" {
  description = "Namespace for the routers workloads."
  type        = string
  default     = "routers"
}

variable "dependency_namespace" {
  description = "Namespace for NATS, Valkey and observability. Kept apart so a teardown of one does not touch the other."
  type        = string
  default     = "routers-dev"
}

# --- Sizing, matching ../platform -------------------------------------------

variable "shards_file" {
  type    = string
  default = "../platform/shards.txt"
}

variable "shard_precision" {
  type    = number
  default = 4
}

variable "cell_precision" {
  type    = number
  default = 2
}

variable "coverage_cells" {
  type    = list(string)
  default = []
}

variable "throughput_target_eps" {
  type    = number
  default = 20000
}

variable "vertical_profile" {
  type    = string
  default = "standard"
}

variable "historian_queue_group" {
  type    = bool
  default = true
}

variable "machines" {
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

# --- From the ../platform outputs -------------------------------------------

variable "image_registry" {
  description = "`tofu -chdir=../platform output -raw image_registry`."
  type        = string
}

variable "shard_bucket" {
  description = "`tofu -chdir=../platform output -raw shard_bucket`."
  type        = string
}

variable "shard_cache_service_account_email" {
  description = "`tofu -chdir=../platform output -raw shard_cache_service_account_email`."
  type        = string
}

variable "workload_service_account" {
  description = "Kubernetes service account the matchers run as. Must match the platform module's workload_identity_service_account, or the binding does not apply to these pods."
  type        = string
  default     = "routers-matcher"
}

# --- Images -----------------------------------------------------------------

variable "image_tag" {
  description = "Pin a digest or an immutable tag. With one replica per shard a bad image takes that shard offline with nothing to compare against."
  type        = string
}

variable "image_pull_policy" {
  type    = string
  default = "IfNotPresent"
}

variable "valkey_image" {
  description = <<-EOT
    Worth setting. The Bitnami chart defaults to a floating `latest` tag, and
    Bitnami moved its free public catalogue to `bitnamilegacy/*` during 2025, so
    the default may not resolve to the image you expect.
  EOT
  type = object({
    registry   = optional(string, "")
    repository = optional(string, "")
    tag        = optional(string, "")
  })
  default = {}
}
