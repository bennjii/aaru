variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "australia-southeast1"
}

variable "env" {
  description = "Environment name, used as a prefix on service accounts and as a label."
  type        = string
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

# --- From the registry and shard-cache units --------------------------------

variable "image_repository" {
  description = "The repository the nodes pull from, from the registry unit."
  type = object({
    location = string
    name     = string
  })
}

variable "shard_cache_service_account_id" {
  description = "Full resource name of the shard cache reader, from the shard-cache unit. The matcher pods impersonate it through Workload Identity."
  type        = string
}

variable "workload_namespace" {
  description = "Namespace the realtime unit deploys into. The Workload Identity binding is by namespace and service account name, so it must match that unit."
  type        = string
  default     = "routers"
}

variable "workload_service_account" {
  description = "Kubernetes service account the realtime workloads run as. Must match the realtime unit, or the binding does not apply to its pods."
  type        = string
  default     = "routers-matcher"
}

variable "deletion_protection" {
  type    = bool
  default = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
