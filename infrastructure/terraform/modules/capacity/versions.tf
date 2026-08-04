terraform {
  # 1.9 for validation blocks that reference another variable: shard_precision
  # is checked against both `shards` and the precision compiled into the
  # binaries, and `streams` against the partition count.
  required_version = ">= 1.9"
}
