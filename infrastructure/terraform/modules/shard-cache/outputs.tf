output "bucket" {
  value = google_storage_bucket.shards.name
}

output "reader_service_account" {
  description = "The account matcher pods impersonate. `id` is what the Workload Identity binding in the platform module needs; `email` is the chart's iam.gke.io/gcp-service-account annotation."
  value = {
    id    = google_service_account.reader.name
    email = google_service_account.reader.email
  }
}

output "publisher_service_account_email" {
  description = "The account `generate-shards` uploads as."
  value       = google_service_account.publisher.email
}
