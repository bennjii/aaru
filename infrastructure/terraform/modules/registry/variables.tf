variable "project_id" {
  type = string
}

variable "region" {
  description = "Repository location. Same region as the cluster, so pulls stay regional."
  type        = string
}

variable "env" {
  type = string
}

variable "repository_id" {
  description = "Name of the Docker repository that holds the matcher and orchestrator images."
  type        = string
  default     = "routers"
}

variable "immutable_tags" {
  description = "Refuse to move a tag that already exists. Worth leaving on: with one replica per shard, a bad image takes that shard offline and a moved tag leaves nothing to roll back to."
  type        = bool
  default     = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
