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
  description = "Namespace for the routers workloads. Must match the platform unit's `workload_namespace`, which is what the Workload Identity binding names."
  type        = string
  default     = "routers"
}

variable "chart_path" {
  description = "Absolute path to the routers-realtime chart. Terragrunt passes `<repo>/infrastructure/chart`; the unit is copied into a cache directory before it runs, so a relative path would not survive."
  type        = string
}

# --- From the dependencies unit ---------------------------------------------

variable "nats_url" {
  type = string
}

variable "valkey_urls" {
  description = "The whole Valkey fleet. Checked against this unit's own model, so a dependencies apply from a different sizing is caught at plan."
  type        = list(string)
}

variable "otlp_url" {
  type    = string
  default = ""
}

# --- From the platform unit -------------------------------------------------

variable "image_registry" {
  type = string
}

variable "shard_bucket" {
  type = string
}

variable "shard_cache_service_account_email" {
  type = string
}

variable "workload_service_account" {
  description = "Kubernetes service account the workloads run as. Must match the platform unit's `workload_service_account`, or the Workload Identity binding does not apply to these pods."
  type        = string
  default     = "routers-matcher"
}

variable "pool_node_selectors" {
  description = "nodeSelector per pool, from the platform unit. Must contain `matcher` and `pipeline`."
  type        = map(map(string))
}

variable "pool_tolerations" {
  description = "Tolerations per pool, from the platform unit."
  type = map(list(object({
    key      = string
    operator = string
    value    = string
    effect   = string
  })))
}

# --- Images -----------------------------------------------------------------

variable "image_tag" {
  description = "Pin a digest or an immutable tag, so a rollout is reproducible and a rollback has something to return to."
  type        = string

  validation {
    condition     = var.image_tag != ""
    error_message = "image_tag is empty. Set ROUTERS_IMAGE_TAG, or pin the tag in live/<env>/realtime/terragrunt.hcl."
  }
}

variable "image_pull_policy" {
  type    = string
  default = "IfNotPresent"
}
