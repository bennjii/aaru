output "repository" {
  description = "Location and name, for a consumer that grants itself pull access."
  value = {
    location = google_artifact_registry_repository.images.location
    name     = google_artifact_registry_repository.images.name
    id       = google_artifact_registry_repository.images.id
  }
}

output "image_registry" {
  description = "Registry prefix for the chart's image.registry."
  value       = "${google_artifact_registry_repository.images.location}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.images.repository_id}"
}

output "publisher_service_account_email" {
  description = "The account CI pushes as."
  value       = google_service_account.publisher.email
}
