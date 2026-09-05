# A public copy of the shard cache behind Cloud CDN, for browser consumers.
#
# The WebAssembly component (libs/routers_wasm) `fetch()`es one `.shard.rt`
# blob per visible cell. That needs a public, CORS-enabled origin, which the
# matchers' bucket must not be: its objects are read through Workload Identity
# and public access prevention is enforced on it. So this is a second bucket,
# published to by the same account, fronted by a global external Application
# Load Balancer with a backend bucket and CDN on.
#
# Cluster-free, and not free: a global forwarding rule is billed hourly (about
# USD 18/month for the first five) plus CDN egress. Still a rounding error
# against one node, and the whole thing can be off in an environment that has
# no browser consumers — see the unit's `cdn` input.
#
# HTTPS needs a hostname you own, pointed at the address this creates, because
# the managed certificate provisions only once the DNS resolves. Without a
# hostname the origin is plain HTTP on the address.

resource "google_storage_bucket" "public" {
  project  = var.project_id
  name     = var.bucket_name
  location = var.region

  storage_class               = "STANDARD"
  force_destroy               = !var.deletion_protection
  uniform_bucket_level_access = true

  # This bucket exists to be public. The project-level default may be
  # "enforced"; "inherited" takes that default, so if the organisation forbids
  # public buckets the apply fails here rather than serving nothing.
  public_access_prevention = "inherited"

  # Browsers refuse the bytes without this. GET and HEAD only: nothing writes
  # through the CDN.
  cors {
    origin          = var.cors_origins
    method          = ["GET", "HEAD"]
    response_header = ["Content-Type", "Content-Length", "ETag", "Cache-Control", "Accept-Ranges", "Content-Range"]
    max_age_seconds = 3600
  }

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
    component = "routers-shard-cdn"
  })
}

resource "google_storage_bucket_iam_member" "public_read" {
  bucket = google_storage_bucket.public.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

resource "google_storage_bucket_iam_member" "publisher" {
  bucket = google_storage_bucket.public.name
  role   = "roles/storage.objectAdmin"
  member = "serviceAccount:${var.publisher_service_account_email}"
}

resource "google_compute_backend_bucket" "shards" {
  project     = var.project_id
  name        = "${var.name}-shards"
  bucket_name = google_storage_bucket.public.name
  enable_cdn  = true

  cdn_policy {
    # A shard file is regenerated under the same name, so it is not immutable
    # content: cache it, but for hours, not days, and let a generation roll
    # through the edges without a purge.
    cache_mode  = "CACHE_ALL_STATIC"
    default_ttl = var.cache_ttl_seconds
    max_ttl     = var.cache_ttl_seconds * 4
    client_ttl  = var.cache_ttl_seconds

    # A viewport asks for cells that do not exist (sea, desert). Caching the
    # 404 keeps those off the bucket.
    negative_caching = true
  }
}

resource "google_compute_url_map" "shards" {
  project         = var.project_id
  name            = "${var.name}-shards"
  default_service = google_compute_backend_bucket.shards.id
}

resource "google_compute_global_address" "shards" {
  project = var.project_id
  name    = "${var.name}-shards"
}

# --- HTTP, always ----------------------------------------------------------

resource "google_compute_target_http_proxy" "shards" {
  project = var.project_id
  name    = "${var.name}-shards-http"
  url_map = google_compute_url_map.shards.id
}

resource "google_compute_global_forwarding_rule" "http" {
  project               = var.project_id
  name                  = "${var.name}-shards-http"
  target                = google_compute_target_http_proxy.shards.id
  ip_address            = google_compute_global_address.shards.address
  port_range            = "80"
  load_balancing_scheme = "EXTERNAL_MANAGED"
}

# --- HTTPS, with a hostname -------------------------------------------------

resource "google_compute_managed_ssl_certificate" "shards" {
  count = var.hostname == "" ? 0 : 1

  project = var.project_id
  name    = "${var.name}-shards"

  managed {
    domains = [var.hostname]
  }
}

resource "google_compute_target_https_proxy" "shards" {
  count = var.hostname == "" ? 0 : 1

  project          = var.project_id
  name             = "${var.name}-shards-https"
  url_map          = google_compute_url_map.shards.id
  ssl_certificates = [google_compute_managed_ssl_certificate.shards[0].id]
}

resource "google_compute_global_forwarding_rule" "https" {
  count = var.hostname == "" ? 0 : 1

  project               = var.project_id
  name                  = "${var.name}-shards-https"
  target                = google_compute_target_https_proxy.shards[0].id
  ip_address            = google_compute_global_address.shards.address
  port_range            = "443"
  load_balancing_scheme = "EXTERNAL_MANAGED"
}
