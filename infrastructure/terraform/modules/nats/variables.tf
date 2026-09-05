variable "namespace" {
  description = "Namespace for the cluster. Kept apart from the routers workloads so a teardown of one does not touch the other."
  type        = string
}

variable "create_namespace" {
  description = "Whether Helm creates the namespace."
  type        = bool
  default     = true
}

# --- Topology ---------------------------------------------------------------

variable "replicas" {
  description = <<-EOT
    Servers in the single NATS cluster, from the capacity module's
    `nats.replicas_total`. Sized by whichever binds first — routed core
    deliveries or persisted JetStream writes — and rounded up to odd, because a
    stream's leader is elected.
  EOT
  type        = number

  validation {
    condition     = var.replicas >= 1
    error_message = "replicas must be at least 1."
  }
}

variable "cpu_millis" {
  description = "CPU request per server. Core routing is a network loop, but JetStream adds raft and file I/O on top, so this buys both."
  type        = number
  default     = 4000
}

variable "memory_mib" {
  description = "Memory request and limit per server."
  type        = number
  default     = 8192
}

# --- JetStream file store ---------------------------------------------------

variable "jetstream_file_store_gib" {
  description = <<-EOT
    File store PVC per server, from the capacity module's `nats.file_store_gib`.

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

    Hyperdisk, because it provisions performance independently of capacity — a
    pd-* class would tie the broker's write ceiling to its retention window,
    and C4 does not offer Persistent Disk in any case.

    Balanced rather than Extreme, for two independent reasons:

      Extreme is unavailable at this node size. C4 requires at least 96 vCPUs
      to attach a Hyperdisk Extreme volume, and the infra pool is nowhere near
      that. Reaching it would mean sizing the pool for the disk rather than for
      the brokers.

      Extreme is also the wrong shape. It provisions IOPS only: throughput
      comes as 250 MiB/s per 1,000 IOPS, capped at 5,000. This workload is
      throughput-bound — JetStream appends sequentially — so Balanced, which
      provisions both, targets the constraint that actually binds. Balanced
      reaches 160,000 IOPS and 2,400 MiB/s per volume, well past what the
      streams need.
  EOT
  type        = string
  default     = "hyperdisk-balanced"

  validation {
    condition     = startswith(var.jetstream_disk_type, "hyperdisk-")
    error_message = "jetstream_disk_type must be a hyperdisk type: a pd-* volume's performance scales with its size, which makes the write ceiling depend on the retention window rather than on the traffic."
  }
}

variable "jetstream_instance_iops_limit" {
  description = <<-EOT
    IOPS the NATS pod's *node* can sustain across every disk attached to it,
    from the machine series' Hyperdisk performance limits.

    A volume cannot reach its provisioned performance on an instance that does
    not support that level, and nothing reports the shortfall: the disk simply
    runs at the instance's ceiling while the bill reflects the provisioning.
  EOT
  type        = number
  default     = 100000
}

variable "jetstream_instance_throughput_limit_mib" {
  description = "MiB/s the NATS pod's node can sustain across every attached disk, including its boot disk. The same silent-shortfall applies as for IOPS."
  type        = number
  default     = 1600
}

variable "jetstream_provisioned_iops" {
  description = "IOPS provisioned per file store volume. One volume per server, so this is per server."
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

# --- Scheduling -------------------------------------------------------------

variable "node_selector" {
  description = "Node labels pinning the servers to their pool."
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
  description = "Pinned nats chart version. A floating version turns an unrelated apply into an unplanned upgrade. Confirmed upstream on 2026-07-31."
  type        = string
  default     = "2.14.2"
}

variable "release_timeout" {
  description = "Release timeout in seconds."
  type        = number
  default     = 900
}

variable "wait_for_rollout" {
  description = "Whether Helm waits for pods to become Ready. False keeps an apply from serialising behind the slowest StatefulSet."
  type        = bool
  default     = false
}
