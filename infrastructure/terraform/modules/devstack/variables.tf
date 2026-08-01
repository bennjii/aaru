variable "namespace" {
  description = "Namespace for the dependencies. Kept apart from the routers workloads so a teardown of one does not touch the other."
  type        = string
  default     = "routers-dev"
}

variable "create_namespace" {
  description = "Whether Helm creates the namespace."
  type        = bool
  default     = true
}

# --- Topology ---------------------------------------------------------------

variable "nats_replicas" {
  description = "Servers in the single core NATS cluster, from the capacity module's `nats.replicas_total`. Sized by total delivery rate, not by cell count."
  type        = number

  validation {
    condition     = var.nats_replicas >= 1
    error_message = "nats_replicas must be at least 1."
  }
}

variable "valkey_primaries" {
  description = "Primaries in the Valkey fleet, from the capacity module. Each is an independent release. Clients place a vehicle by rendezvous hash over the primary URLs."
  type        = number

  validation {
    condition     = var.valkey_primaries >= 1
    error_message = "valkey_primaries must be at least 1."
  }
}

variable "valkey_client_mode" {
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
    condition     = contains(["single", "pooled-hash"], var.valkey_client_mode)
    error_message = "valkey_client_mode must be 'single' or 'pooled-hash'."
  }
}

variable "valkey_io_threads" {
  description = "Valkey `io-threads`. The 1B RPS run used 6 on an 8-core node, leaving 2 cores for network interrupt affinity."
  type        = number
  default     = 6
}

variable "valkey_persistence" {
  description = "Whether primaries keep a PVC. Off by default: the keyspace is a rolling window trimmed to the historian's HISTORY entries, and it rebuilds from the event stream within seconds."
  type        = bool
  default     = false
}

# --- Resources --------------------------------------------------------------

variable "nats_cpu_millis" {
  description = "CPU request per NATS server."
  type        = number
  default     = 4000
}

variable "nats_memory_mib" {
  description = "Memory request and limit per NATS server."
  type        = number
  default     = 8192
}

variable "valkey_cpu_millis" {
  description = "CPU request per Valkey pod."
  type        = number
  default     = 2000
}

variable "valkey_memory_mib" {
  description = "Memory request and limit per Valkey pod."
  type        = number
  default     = 4096
}

variable "valkey_replicas_per_primary" {
  description = "Replicas per primary, for failover only. The pipeline never reads a replica: the orchestrator's fetch must see the historian's latest write. Zero installs a standalone primary."
  type        = number
  default     = 1
}

variable "collector_replicas" {
  description = "otel-collector replicas, sized from the sampled span rate."
  type        = number
  default     = 1
}

variable "collector_cpu_millis" {
  description = "CPU request per collector replica."
  type        = number
  default     = 2000
}

variable "collector_memory_mib" {
  description = "Memory request and limit per collector replica."
  type        = number
  default     = 2048
}

# --- Images -----------------------------------------------------------------

variable "valkey_image" {
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

# --- Observability ----------------------------------------------------------

variable "observability_enabled" {
  description = "Whether to install kube-prometheus-stack. The collector is installed regardless, because it is the services' only telemetry path."
  type        = bool
  default     = true
}

variable "grafana_anonymous_admin" {
  description = "Logs every visitor in as Admin with no password. Developer machines only: anyone who reaches the service is an administrator."
  type        = bool
  default     = false
}

variable "metrics_flush_interval" {
  description = "How often the spanmetrics connector flushes. The collector's own default is a leisurely 60s, which makes a dashboard useless during an incident."
  type        = string
  default     = "5s"
}

variable "span_metrics_dimensions" {
  description = "Span attributes promoted to metric labels. Everything else stays out of Prometheus, so the services can attribute spans freely."
  type        = list(string)
  default     = ["outcome", "severity", "reason", "continuation", "subject"]
}

# --- Scheduling -------------------------------------------------------------

variable "node_selector" {
  description = "Node labels pinning the dependencies to their pool."
  type        = map(string)
  default     = {}
}

variable "tolerations" {
  description = "Tolerations for the dependency pool's taint."
  type        = list(any)
  default     = []
}

variable "observability_node_selector" {
  description = "Node labels for Prometheus, Grafana and the collector, which normally sit on a different pool from NATS and Valkey."
  type        = map(string)
  default     = {}
}

variable "observability_tolerations" {
  description = "Tolerations for the observability pool's taint."
  type        = list(any)
  default     = []
}

# --- Chart versions ---------------------------------------------------------

variable "chart_versions" {
  description = "Pinned chart versions. Floating versions turn an unrelated `tofu apply` into an unplanned dependency upgrade. Confirmed upstream on 2026-07-31."
  type = object({
    nats       = optional(string, "2.14.2")
    valkey     = optional(string, "6.2.4")
    prometheus = optional(string, "88.0.1")
    collector  = optional(string, "0.165.0")
  })
  default = {}
}

variable "release_timeout" {
  description = "Per-release timeout in seconds."
  type        = number
  default     = 900
}

variable "wait_for_rollout" {
  description = "Whether Helm waits for pods to become Ready. False keeps a large apply from serialising behind the slowest StatefulSet."
  type        = bool
  default     = false
}
