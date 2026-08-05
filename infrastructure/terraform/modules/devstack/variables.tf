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
  description = <<-EOT
    Servers in the single NATS cluster, from the capacity module's
    `nats.replicas_total`. Sized by whichever binds first — routed core
    deliveries or persisted JetStream writes — and rounded up to odd, because a
    stream's leader is elected.
  EOT
  type        = number

  validation {
    condition     = var.nats_replicas >= 1
    error_message = "nats_replicas must be at least 1."
  }
}

variable "jetstream_file_store_gib" {
  description = <<-EOT
    File store PVC per NATS server, from the capacity module's
    `nats.file_store_gib`.

    Holds the raw work queues — which are the ingest buffer, since a work queue
    deletes on ack and so holds only the backlog — plus the matched stream for
    its whole retention. The matched stream dominates: emissions carry the
    entire cut trip rather than a delta, so its byte rate is a multiple of
    ingest's.
  EOT
  type        = number

  validation {
    condition     = var.jetstream_file_store_gib >= 1
    error_message = "jetstream_file_store_gib must be at least 1."
  }
}

variable "jetstream_storage_class" {
  description = <<-EOT
    An existing StorageClass to bind the file store PVCs to. Empty — the
    default — makes this module create one instead, sized from the demand
    variables below.

    Do not point this at the cluster default. GKE's is pd-balanced, whose
    throughput and IOPS scale with capacity, so the broker's disk speed becomes
    a side effect of how much retention was configured.
  EOT
  type        = string
  default     = ""
}

variable "jetstream_storage_class_name" {
  description = "Name of the StorageClass this module creates when `jetstream_storage_class` is empty."
  type        = string
  default     = "routers-jetstream"
}

variable "jetstream_disk_type" {
  description = <<-EOT
    Disk type behind the file store.

    Hyperdisk, because it provisions IOPS and throughput independently of
    capacity — a pd-* class would tie the broker's write ceiling to its
    retention window. `hyperdisk-balanced` covers this workload;
    `hyperdisk-extreme` exists for a measured IOPS wall that balanced cannot
    reach.
  EOT
  type        = string
  default     = "hyperdisk-balanced"

  validation {
    condition     = startswith(var.jetstream_disk_type, "hyperdisk-")
    error_message = "jetstream_disk_type must be a hyperdisk type: a pd-* volume's performance scales with its size, which makes the write ceiling depend on the retention window rather than on the traffic."
  }
}

variable "jetstream_provisioned_iops" {
  description = "IOPS provisioned per file store volume. One volume per NATS server, so this is per server."
  type        = number
  default     = 30000
}

variable "jetstream_provisioned_throughput_mib" {
  description = "Throughput in MiB/s provisioned per file store volume. This is usually the binding one rather than IOPS: JetStream appends sequentially, so the bytes matter more than the operation count."
  type        = number
  default     = 750
}

variable "jetstream_required_iops" {
  description = "IOPS the streams will actually need per server, from the capacity model. Checked against the provisioned figure."
  type        = number
  default     = 0
}

variable "jetstream_required_throughput_mib" {
  description = "MiB/s the streams will actually write per server, from the capacity model. Checked against the provisioned figure."
  type        = number
  default     = 0
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
  description = "Whether primaries keep a PVC. Off by default: the keyspace is a rolling window trimmed to HISTORY entries per vehicle, and the orchestrator rebuilds a vehicle's tail as it re-warms that lane."
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
  description = "Replicas per primary, for failover only. The pipeline never reads a replica: a warming lane must see the tail its own pod wrote. Zero installs a standalone primary."
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
