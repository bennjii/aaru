# Pull access for the nodes. The repository itself is the registry module's;
# this binding lives here because the node service account does.

resource "google_artifact_registry_repository_iam_member" "nodes_pull" {
  project    = var.project_id
  location   = var.image_repository.location
  repository = var.image_repository.name
  role       = "roles/artifactregistry.reader"
  member     = "serviceAccount:${google_service_account.nodes.email}"
}
