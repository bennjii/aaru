terraform {
  # 1.9 for validation blocks that reference another variable: shard_precision
  # is checked against shards, and cell_precision against shard_precision.
  required_version = ">= 1.9"
}
