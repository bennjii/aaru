variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "australia-southeast1"
}

variable "cluster_name" {
  description = "Cluster from the platform unit. Looked up by data source, so that unit must be applied first."
  type        = string
  default     = "routers"
}

variable "namespace" {
  description = "Namespace for NATS, Valkey and observability. Kept apart from the realtime unit's so a teardown of one does not touch the other."
  type        = string
  default     = "routers-dev"
}

# --- Placement, from the platform unit --------------------------------------

variable "pool_node_selectors" {
  description = "nodeSelector per pool, from the platform unit's output of the same name. Must contain `infra` and `system`."
  type        = map(map(string))
}

variable "pool_tolerations" {
  description = "Tolerations per pool, from the platform unit's output of the same name."
  type = map(list(object({
    key      = string
    operator = string
    value    = string
    effect   = string
  })))
}

# --- JetStream storage ------------------------------------------------------

variable "jetstream_provisioned_throughput_mib" {
  description = <<-EOT
    MiB/s provisioned per NATS server's file store volume.

    Set above the model's derived demand, not equal to it: the derived figure
    is a mean, and ingest is burstier than a mean — a matcher shard recovering
    from a restart re-drives its backlog as fast as the orchestrators can push
    it. The nats module fails the plan if this drops below what the model needs.
  EOT
  type        = number
  default     = 750
}

variable "jetstream_provisioned_iops" {
  description = "IOPS provisioned per file store volume. Rarely the binding constraint — JetStream appends sequentially, so throughput runs out first."
  type        = number
  default     = 30000
}

variable "jetstream_instance_iops_limit" {
  description = <<-EOT
    IOPS the infra pool's machine type sustains across every disk on the node,
    boot disk included. Caps what any volume attached to it can actually
    deliver, so provisioning past this would be bought and not received.
    c4-standard-16 sustains 100,000 IOPS; change this with the machine type in
    sizing.hcl.
  EOT
  type        = number
  default     = 100000
}

variable "jetstream_instance_throughput_limit_mib" {
  description = "MiB/s the infra pool's machine type sustains across every attached disk. c4-standard-16 sustains 1,600 MiB/s."
  type        = number
  default     = 1600
}

# --- Images -----------------------------------------------------------------

variable "valkey_image" {
  description = <<-EOT
    Worth setting. The Bitnami chart defaults to a floating `latest` tag, and
    Bitnami moved its free public catalogue to `bitnamilegacy/*` during 2025, so
    the default may not resolve to the image you expect. The valkey module warns
    on every plan until the tag is pinned.
  EOT
  type = object({
    registry   = optional(string, "")
    repository = optional(string, "")
    tag        = optional(string, "")
  })
  default = {}
}

# --- Observability ----------------------------------------------------------

variable "prometheus_enabled" {
  description = "Whether to install kube-prometheus-stack. The collector is installed regardless."
  type        = bool
  default     = true
}

variable "grafana_anonymous_admin" {
  description = "Logs every Grafana visitor in as Admin. Developer clusters only."
  type        = bool
  default     = false
}
