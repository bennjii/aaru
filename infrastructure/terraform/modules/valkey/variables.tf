variable "namespace" {
  description = "Namespace for the fleet. Kept apart from the routers workloads so a teardown of one does not touch the other."
  type        = string
}

variable "create_namespace" {
  description = "Whether Helm creates the namespace."
  type        = bool
  default     = true
}

# --- Topology ---------------------------------------------------------------

variable "primaries" {
  description = "Primaries in the fleet, from the capacity module. Each is an independent release. Clients place a vehicle by rendezvous hash over the primary URLs."
  type        = number

  validation {
    condition     = var.primaries >= 1
    error_message = "primaries must be at least 1."
  }
}

variable "replicas_per_primary" {
  description = "Replicas per primary, for failover only. The pipeline never reads a replica: a warming lane must see the tail its own pod wrote. Zero installs a standalone primary."
  type        = number
  default     = 1
}

variable "client_mode" {
  description = <<-EOT
    `pooled-hash` is what the code does: one multiplexed connection per
    primary, and a vehicle placed by rendezvous hash over the URLs. Order does
    not matter, and a resize moves about 1/N of vehicles.

    `single` uses one connection, so only the first URL is reached. Valid only
    with one primary.
  EOT
  type        = string
  default     = "pooled-hash"

  validation {
    condition     = contains(["single", "pooled-hash"], var.client_mode)
    error_message = "client_mode must be 'single' or 'pooled-hash'."
  }
}

variable "io_threads" {
  description = "Valkey `io-threads`. The 1B RPS run used 6 on an 8-core node, leaving 2 cores for network interrupt affinity."
  type        = number
  default     = 6
}

variable "persistence" {
  description = "Whether primaries keep a PVC. Off by default: the keyspace is a rolling window trimmed to HISTORY entries per vehicle, and the orchestrator rebuilds a vehicle's tail as it re-warms that lane."
  type        = bool
  default     = false
}

# --- Resources --------------------------------------------------------------

variable "cpu_millis" {
  description = "CPU request per pod. No CPU limit is set: a CFS cap would freeze the command loop in 100ms windows."
  type        = number
  default     = 2000
}

variable "memory_mib" {
  description = "Memory request and limit per pod."
  type        = number
  default     = 4096
}

# --- Images -----------------------------------------------------------------

variable "image" {
  description = <<-EOT
    Overrides the Valkey image. Worth setting: the chart defaults to
    `registry-1.docker.io/bitnami/valkey:latest` and its metrics sidecar to
    `bitnami/redis-exporter:latest`. Floating tags make a deploy
    unreproducible, and Bitnami moved its free public catalogue to
    `bitnamilegacy/*` during 2025, so the default may not resolve. Mirror a
    pinned digest into Artifact Registry. Empty fields take the chart default.
  EOT
  type = object({
    registry   = optional(string, "")
    repository = optional(string, "")
    tag        = optional(string, "")
  })
  default = {}
}

# --- Scheduling -------------------------------------------------------------

variable "node_selector" {
  description = "Node labels pinning the fleet to its pool."
  type        = map(string)
  default     = {}
}

variable "tolerations" {
  description = "Tolerations for that pool's taint."
  type        = list(any)
  default     = []
}

# --- Release ----------------------------------------------------------------

variable "chart_version" {
  description = "Pinned bitnamicharts/valkey version. Confirmed upstream on 2026-07-31."
  type        = string
  default     = "6.2.4"
}

variable "release_timeout" {
  description = "Per-release timeout in seconds."
  type        = number
  default     = 900
}

variable "wait_for_rollout" {
  description = "Whether Helm waits for pods to become Ready."
  type        = bool
  default     = false
}
