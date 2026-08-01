# Generates the synthetic shard list the tests run against.
#
# It lives in a module because a `tofu test` variables block cannot call
# functions, and 1120 shards is not something to write out by hand. Run it with
# `command = apply` — a plan leaves the outputs unknown.

variable "cells" {
  type = list(string)
}

variable "groups_per_cell" {
  type    = number
  default = 8
}

variable "shards_per_group" {
  type    = number
  default = 7
}

output "shards" {
  value = flatten([
    for cell in var.cells : [
      for group in range(var.groups_per_cell) : [
        for i in range(var.shards_per_group) : format("%s%d%03d", cell, group, i)
      ]
    ]
  ])
}
