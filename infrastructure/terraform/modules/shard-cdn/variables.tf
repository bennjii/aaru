variable "project_id" {
  type = string
}

variable "region" {
  description = "Location of the public bucket. The load balancer itself is global."
  type        = string
}

variable "env" {
  type = string
}

variable "name" {
  description = "Prefix for the load balancer resources, which are project-global and so need an environment in the name."
  type        = string
}

variable "bucket_name" {
  description = "The public bucket. Distinct from the matchers' bucket, which stays private."
  type        = string
}

variable "publisher_service_account_email" {
  description = "Account that uploads shard generations here, normally the shard-cache module's publisher."
  type        = string
}

variable "hostname" {
  description = "DNS name for HTTPS, pointed at the address this creates. Empty serves plain HTTP on the address only."
  type        = string
  default     = ""
}

variable "cors_origins" {
  description = "Origins allowed to fetch shards from a browser. `*` for a public dataset; the viewer's origins for anything else."
  type        = list(string)
  default     = ["*"]
}

variable "cache_ttl_seconds" {
  description = "How long an edge serves a shard before revalidating. A new generation reaches every client within this."
  type        = number
  default     = 3600
}

variable "keep_versions" {
  type    = number
  default = 3
}

variable "deletion_protection" {
  type    = bool
  default = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
