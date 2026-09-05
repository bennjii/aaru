# Artifact Registry for the matcher and orchestrator images, and the account
# that pushes to it.
#
# Cluster-free: nothing here needs GKE, so the repository can exist, take
# pushes from CI and be worked on without a cluster running. The node pull
# binding lives in the platform module, which owns the node service account.

resource "google_artifact_registry_repository" "images" {
  project       = var.project_id
  location      = var.region
  repository_id = var.repository_id
  format        = "DOCKER"
  description   = "routers realtime pipeline images"

  docker_config {
    # A tag that can be moved is a tag that cannot be rolled back to. With one
    # replica per shard there is no second pod to compare a bad image against.
    immutable_tags = var.immutable_tags
  }

  labels = merge(var.labels, {
    env       = var.env
    component = "routers-registry"
  })
}

# The publishing identity for CI. Writer on this repository and nothing else;
# authenticate it through Workload Identity Federation from the CI provider
# rather than with a key.
resource "google_service_account" "publisher" {
  project      = var.project_id
  account_id   = "${var.env}-image-publisher"
  display_name = "Image publisher (${var.repository_id})"
}

resource "google_artifact_registry_repository_iam_member" "publisher_write" {
  project    = var.project_id
  location   = google_artifact_registry_repository.images.location
  repository = google_artifact_registry_repository.images.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:${google_service_account.publisher.email}"
}
