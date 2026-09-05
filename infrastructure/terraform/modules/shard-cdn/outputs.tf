output "bucket" {
  description = "The public bucket `generate-shards` publishes to for browsers."
  value       = google_storage_bucket.public.name
}

output "address" {
  description = "The global address. Point the hostname's A record here."
  value       = google_compute_global_address.shards.address
}

output "base_url" {
  description = "Where a browser fetches `<shard>.shard.rt` from."
  value       = var.hostname == "" ? "http://${google_compute_global_address.shards.address}" : "https://${var.hostname}"
}
