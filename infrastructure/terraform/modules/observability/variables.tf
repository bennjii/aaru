variable "namespace" {
  description = "Namespace for the collector and, when enabled, kube-prometheus-stack."
  type        = string
}

variable "create_namespace" {
  description = "Whether Helm creates the namespace."
  type        = bool
  default     = true
}

# --- Collector --------------------------------------------------------------

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

# --- Prometheus and Grafana -------------------------------------------------

variable "prometheus_enabled" {
  description = "Whether to install kube-prometheus-stack. The collector is installed regardless, because it is the services' only telemetry path."
  type        = bool
  default     = true
}

variable "grafana_anonymous_admin" {
  description = "Logs every visitor in as Admin with no password. Developer machines only: anyone who reaches the service is an administrator."
  type        = bool
  default     = false
}

# --- Scheduling -------------------------------------------------------------

variable "node_selector" {
  description = "Node labels for Prometheus, Grafana and the collector, which normally sit on a different pool from NATS and Valkey."
  type        = map(string)
  default     = {}
}

variable "tolerations" {
  description = "Tolerations for that pool's taint."
  type        = list(any)
  default     = []
}

# --- Release ----------------------------------------------------------------

variable "chart_versions" {
  description = "Pinned chart versions. Floating versions turn an unrelated apply into an unplanned dependency upgrade. Confirmed upstream on 2026-07-31."
  type = object({
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
  description = "Whether Helm waits for pods to become Ready."
  type        = bool
  default     = false
}
