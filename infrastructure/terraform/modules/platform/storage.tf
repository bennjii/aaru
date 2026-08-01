# Shard cache bucket. Matcher pods mount it read-only through the GCS FUSE CSI
# driver, so the `<shard>.shard.rt` files are neither baked into the image nor
# synced onto a hostPath.

resource "google_storage_bucket" "shards" {
  project  = var.project_id
  name     = var.shard_bucket_name
  location = var.region

  # Same region as the cluster. A matcher reads its whole shard file at
  # startup, so a cross-region read costs latency and egress on every restart.
  storage_class = "STANDARD"

  force_destroy               = !var.deletion_protection
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"

  # A shard file is regenerated, not edited. Versioning is what makes a bad
  # generation recoverable while matchers are already reading it.
  versioning {
    enabled = true
  }

  lifecycle_rule {
    condition {
      num_newer_versions = var.shard_bucket_keep_versions
    }
    action {
      type = "Delete"
    }
  }

  labels = merge(var.labels, {
    env       = var.env
    component = "routers-platform"
  })
}

resource "google_service_account" "shard_cache" {
  project      = var.project_id
  account_id   = "${var.env}-shard-cache"
  display_name = "Shard cache reader (${var.cluster_name})"
}

resource "google_storage_bucket_iam_member" "shard_cache_read" {
  bucket = google_storage_bucket.shards.name
  role   = "roles/storage.objectViewer"
  member = "serviceAccount:${google_service_account.shard_cache.email}"
}

# Lets the named Kubernetes service account impersonate the Google one, which
# is what removes the service account key. The binding is by name and holds
# whether or not the chart has created that service account yet.
resource "google_service_account_iam_member" "shard_cache_workload_identity" {
  service_account_id = google_service_account.shard_cache.name
  role               = "roles/iam.workloadIdentityUser"
  member             = "serviceAccount:${var.project_id}.svc.id.goog[${var.workload_identity_namespace}/${var.workload_identity_service_account}]"
}
