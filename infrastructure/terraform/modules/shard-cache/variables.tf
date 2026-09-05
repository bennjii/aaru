variable "project_id" {
  type = string
}

variable "region" {
  description = "Bucket location. Same region as the cluster that mounts it."
  type        = string
}

variable "env" {
  type = string
}

variable "bucket_name" {
  description = "Bucket holding the `<shard>.shard.rt` files. Bucket names are global, so this needs a project or organisation prefix to be unique."
  type        = string
}

variable "keep_versions" {
  description = "Noncurrent shard file versions retained before deletion."
  type        = number
  default     = 3

  validation {
    condition     = var.keep_versions >= 1
    error_message = "keep_versions must be at least 1."
  }
}

variable "deletion_protection" {
  description = "When false the bucket can be destroyed with objects in it."
  type        = bool
  default     = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
