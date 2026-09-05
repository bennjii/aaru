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

variable "repository_id" {
  type    = string
  default = "routers"
}

variable "immutable_tags" {
  type    = bool
  default = true
}

variable "labels" {
  type    = map(string)
  default = {}
}
