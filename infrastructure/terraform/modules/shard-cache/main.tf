# The shard cache: the bucket the matchers mount, and the identities around it.
#
# Cluster-free. The bucket, the reader account and the publisher account exist
# and can be filled before any cluster does; the Workload Identity binding that
# lets a pod impersonate the reader lives in the platform module, because the
# identity pool it names is the cluster's.
#
# Private, deliberately. Matcher pods mount it read-only through the GCS FUSE
# CSI driver, so the `<shard>.shard.rt` files are neither baked into the image
# nor synced onto a hostPath. Browser consumers do not read this bucket: the
# shard-cdn module publishes a public copy for them.

resource "google_storage_bucket" "shards" {
  project  = var.project_id
  name     = var.bucket_name
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
      num_newer_versions = var.keep_versions
    }
    action {
      type = "Delete"
    }
  }

  labels = merge(var.labels, {
    env       = var.env
    component = "routers-shard-cache"
  })
}

# What the matcher pods become, through Workload Identity.
resource "google_service_account" "reader" {
  project      = var.project_id
  account_id   = "${var.env}-shard-cache"
  display_name = "Shard cache reader (${var.env})"
}

resource "google_storage_bucket_iam_member" "reader" {
  bucket = google_storage_bucket.shards.name
  role   = "roles/storage.objectViewer"
  member = "serviceAccount:${google_service_account.reader.email}"
}

# What `generate-shards` uploads as. Object admin on this bucket, and on the
# public copy when the CDN is on, so one identity publishes a generation to
# both.
resource "google_service_account" "publisher" {
  project      = var.project_id
  account_id   = "${var.env}-shard-publisher"
  display_name = "Shard publisher (${var.env})"
}

resource "google_storage_bucket_iam_member" "publisher" {
  bucket = google_storage_bucket.shards.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${google_service_account.publisher.email}"
}
