# Generates the synthetic shard list the tests run against.
#
# It lives in a module because a `tofu test` variables block cannot call
# functions, and a few hundred shards is not something to write out by hand.
# Run it with `command = apply` — a plan leaves the outputs unknown.

variable "prefixes" {
  description = "Leading geohash characters to fan out from, each of length precision - 2."
  type        = list(string)
}

variable "shards_per_prefix" {
  description = "Shards generated under each prefix. Capped at 32 so the last two characters stay a valid geohash pair."
  type        = number
  default     = 32

  validation {
    condition     = var.shards_per_prefix >= 1 && var.shards_per_prefix <= 32
    error_message = "shards_per_prefix must be in 1..32; the geohash alphabet has 32 symbols."
  }
}

locals {
  # The geohash alphabet: base32 without a, i, l or o.
  alphabet = split("", "0123456789bcdefghjkmnpqrstuvwxyz")
}

output "shards" {
  value = flatten([
    for prefix in var.prefixes : [
      for i in range(var.shards_per_prefix) : "${prefix}${local.alphabet[i]}"
    ]
  ])
}
