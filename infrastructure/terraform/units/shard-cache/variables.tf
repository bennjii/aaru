variable "project_id" {
  type = string
}

variable "region" {
  type    = string
  default = "australia-southeast1"
}

variable "env" {
  type = string
}

variable "bucket_name" {
  description = "The private bucket the matchers mount."
  type        = string
}

variable "keep_versions" {
  description = "Noncurrent shard generations retained, on both buckets."
  type        = number
  default     = 3
}

variable "deletion_protection" {
  type    = bool
  default = true
}

variable "cdn" {
  description = <<-EOT
    The public copy for browser consumers of the WebAssembly component.
    `bucket_name` must differ from the private bucket. `hostname` enables
    HTTPS with a managed certificate once its A record points at the address
    this unit outputs; empty serves plain HTTP on the address.
  EOT
  type = object({
    enabled           = bool
    bucket_name       = string
    hostname          = optional(string, "")
    cors_origins      = optional(list(string), ["*"])
    cache_ttl_seconds = optional(number, 3600)
  })
  default = {
    enabled     = false
    bucket_name = ""
  }
}

variable "labels" {
  type    = map(string)
  default = {}
}
