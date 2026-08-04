# Node pools, one per entry in `node_pools`. Shapes come from the capacity
# module; this file only turns them into infrastructure.

resource "google_service_account" "nodes" {
  project      = var.project_id
  account_id   = "${var.env}-gke-nodes"
  display_name = "GKE nodes (${var.cluster_name})"
}

# The default compute service account is Editor on the project. Nodes get their
# own instead, with only what a kubelet needs to report in.
resource "google_project_iam_member" "nodes" {
  for_each = toset([
    "roles/logging.logWriter",
    "roles/monitoring.metricWriter",
    "roles/monitoring.viewer",
    "roles/stackdriver.resourceMetadata.writer",
  ])

  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.nodes.email}"
}

resource "google_container_node_pool" "pool" {
  for_each = var.node_pools

  project  = var.project_id
  name     = each.key
  location = var.region
  cluster  = google_container_cluster.cluster.name

  # Per zone, unlike the totals below. The autoscaler corrects it immediately,
  # so it only decides how many nodes exist before the first scale event.
  initial_node_count = 1

  autoscaling {
    # `total_` prefixed, deliberately. The unprefixed pair is per zone, so on a
    # three-zone regional cluster it would treble every number the capacity
    # module produced.
    total_min_node_count = each.value.min_node_count
    total_max_node_count = each.value.max_node_count

    # Spread nodes over the region's zones rather than filling the cheapest, so
    # a zone loss costs a fraction of each tier rather than all of one. It
    # matters most to the orchestrator fleet: a lost pod's vehicle partitions
    # have no other owner until it reschedules, where a shard's matchers form a
    # queue group whose surviving members absorb the requests.
    location_policy = "BALANCED"
  }

  max_pods_per_node = var.max_pods_per_node

  management {
    auto_repair  = true
    auto_upgrade = true
  }

  upgrade_settings {
    # Add a node before taking one away. max_unavailable > 0 would drain
    # orchestrator pods whose partitions have no other owner, and each
    # replacement re-warms from Valkey every history lane it inherits.
    strategy        = "SURGE"
    max_surge       = 1
    max_unavailable = 0
  }

  node_config {
    machine_type = each.value.machine_type
    spot         = each.value.spot

    disk_size_gb = var.node_disk_size_gb
    disk_type    = var.node_disk_type

    service_account = google_service_account.nodes.email
    # Scopes are the legacy access-control layer and cannot be narrowed usefully
    # once IAM is doing the work; the node's own IAM roles are the real bound.
    oauth_scopes = ["https://www.googleapis.com/auth/cloud-platform"]

    labels = local.pool_node_labels[each.key]

    dynamic "taint" {
      for_each = each.value.taints
      content {
        key    = taint.value.key
        value  = taint.value.value
        effect = taint.value.effect
      }
    }

    dynamic "ephemeral_storage_local_ssd_config" {
      for_each = each.value.local_ssd_count > 0 ? [each.value.local_ssd_count] : []
      content {
        local_ssd_count = ephemeral_storage_local_ssd_config.value
      }
    }

    # Required for Workload Identity. Without it pods reach the legacy metadata
    # server and inherit the node service account.
    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    # Lazily pull image layers, so a matcher starts before its whole image has
    # landed. These images are large and every scale-up event pays for them.
    gcfs_config {
      enabled = true
    }

    shielded_instance_config {
      enable_secure_boot          = true
      enable_integrity_monitoring = true
    }

    resource_labels = merge(var.labels, {
      env       = var.env
      component = "routers-platform"
      pool      = each.key
    })
  }

  lifecycle {
    # The autoscaler owns this after creation.
    ignore_changes = [initial_node_count]

    precondition {
      condition     = !(each.value.spot && length(each.value.taints) == 0)
      error_message = "Pool ${each.key} is spot with no taint, so anything may land on preemptible nodes. That is survivable for a shard's matchers, which are a queue group, and not for the orchestrator fleet, whose pods each own vehicle partitions outright. Make the placement deliberate: add a taint and select for it in the chart."
    }
  }
}
