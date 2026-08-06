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
  description = "Must match ../platform, the shard file entries, and the precision compiled into the binaries."
  type        = number
  default     = 4
}

variable "coverage_cells" {
  type    = list(string)
  default = []
}

variable "throughput_target_eps" {
  description = "Must match ../platform, or the node pools were sized for a different fleet from the one this deploys."
  type        = number
  default     = 800000
}

variable "design_target_eps" {
  description = "Must match ../platform. Only the wire-law quantities are held to it."
  type        = number
  default     = 5000000
}

variable "streams" {
  description = <<-EOT
    Raw JetStream streams, and the value the orchestrator creates them with.

    Must match ../platform exactly, and must never change once events exist: a
    revision is a stream sequence, so remapping partitions to a different stream
    count resets sequence domains and breaks revision comparison across the
    boundary.
  EOT
  type        = number
  default     = 64
}

variable "vertical_profile" {
  type    = string
  default = "standard"
}

# --- JetStream storage ------------------------------------------------------

variable "jetstream_provisioned_throughput_mib" {
  description = <<-EOT
    MiB/s provisioned per NATS server's file store volume.

    Set above the model's derived demand, not equal to it: the derived figure
    is a mean, and ingest is burstier than a mean — a matcher shard recovering
    from a restart re-drives its backlog as fast as the orchestrators can push
    it. The devstack fails the plan if this drops below what the model needs.
  EOT
  type        = number
  default     = 750
}

variable "jetstream_provisioned_iops" {
  description = "IOPS provisioned per file store volume. Rarely the binding constraint — JetStream appends sequentially, so throughput runs out first."
  type        = number
  default     = 30000
}

variable "machines" {
  description = "Machine shape per node pool. Must match ../platform, which built the pools from these; this root only recomputes the model to size what it deploys onto them."
  type = map(object({
    machine_type = string
    vcpu         = number
    memory_gib   = number
  }))

  default = {
    matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
    pipeline = { machine_type = "c4-highcpu-16", vcpu = 16, memory_gib = 32 }
    infra    = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 60 }
    system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 30 }
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
  description = "Kubernetes service account the workloads run as. Must match the platform module's workload_identity_service_account, or the binding does not apply to these pods."
  type        = string
  default     = "routers-matcher"
}

# --- Images -----------------------------------------------------------------

variable "image_tag" {
  description = "Pin a digest or an immutable tag, so a rollout is reproducible and a rollback has something to return to."
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
