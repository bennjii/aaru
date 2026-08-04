# Artifact Registry for the matcher and orchestrator images.

resource "google_artifact_registry_repository" "images" {
  project       = var.project_id
  location      = var.region
  repository_id = var.artifact_repository_id
  format        = "DOCKER"
  description   = "routers realtime pipeline images"

  docker_config {
    # A tag that can be moved is a tag that cannot be rolled back to. With one
    # replica per shard there is no second pod to compare a bad image against.
    immutable_tags = var.immutable_image_tags
  }

  labels = merge(var.labels, {
    env       = var.env
    component = "routers-platform"
  })
}

# Repository-scoped, not project-wide: nodes pull these images and nothing else.
resource "google_artifact_registry_repository_iam_member" "nodes_pull" {
  project    = var.project_id
  location   = google_artifact_registry_repository.images.location
  repository = google_artifact_registry_repository.images.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${google_service_account.nodes.email}"
}
