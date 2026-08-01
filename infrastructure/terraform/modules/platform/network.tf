# VPC, node subnet and egress. Nodes are private, so the only route off the
# cluster is Cloud NAT.

resource "google_compute_network" "vpc" {
  project = var.project_id
  name    = var.network_name

  # Auto mode creates a subnet in every region with ranges we do not control,
  # and those ranges tend to collide with the pod range later.
  auto_create_subnetworks = false

  # Traffic is pod-to-pod within the region; global routing only adds cost.
  routing_mode = "REGIONAL"
}

resource "google_compute_subnetwork" "nodes" {
  project = var.project_id
  name    = local.subnet_name
  region  = var.region
  network = google_compute_network.vpc.id

  # Only nodes draw from the primary range. That is what VPC-native means.
  ip_cidr_range = var.subnet_cidr

  # Nodes have no external IPs, so they reach Google APIs (Artifact Registry,
  # GCS, the metadata server) over the internal path rather than NAT.
  private_ip_google_access = true

  secondary_ip_range {
    range_name    = local.pods_range_name
    ip_cidr_range = var.pods_secondary_cidr
  }

  secondary_ip_range {
    range_name    = local.services_range_name
    ip_cidr_range = var.services_secondary_cidr
  }

  lifecycle {
    # The pod range is fixed for the life of the cluster: enlarging it means
    # rebuilding. Fail at plan time, not at 3am when the autoscaler stops
    # adding nodes because it cannot allocate a pod range.
    precondition {
      condition     = local.pod_range_node_capacity >= local.max_nodes
      error_message = "pods_secondary_cidr (${var.pods_secondary_cidr}) holds ${local.pod_range_node_capacity} nodes at ${var.max_pods_per_node} pods per node, but the pools scale to ${local.max_nodes}. Widen the range by one bit to double the capacity."
    }

    # Overlap is accepted by the API in some orders and then breaks routing.
    precondition {
      condition = !(
        cidrhost(var.subnet_cidr, 0) == cidrhost(var.pods_secondary_cidr, 0) ||
        cidrhost(var.subnet_cidr, 0) == cidrhost(var.services_secondary_cidr, 0) ||
        cidrhost(var.pods_secondary_cidr, 0) == cidrhost(var.services_secondary_cidr, 0)
      )
      error_message = "subnet_cidr, pods_secondary_cidr and services_secondary_cidr must be distinct ranges."
    }
  }
}

resource "google_compute_router" "router" {
  project = var.project_id
  name    = "${local.name_prefix}-router"
  region  = var.region
  network = google_compute_network.vpc.id
}

resource "google_compute_router_nat" "nat" {
  project = var.project_id
  name    = "${local.name_prefix}-nat"
  region  = var.region
  router  = google_compute_router.router.name

  # Private nodes still need the public internet: image layers not in Artifact
  # Registry, GKE addons, OS updates during repair. Without NAT a new node
  # boots and then fails to pull.
  nat_ip_allocate_option             = "AUTO_ONLY"
  source_subnetwork_ip_ranges_to_nat = "ALL_SUBNETWORKS_ALL_IP_RANGES"

  log_config {
    enable = true
    # Successful translations are high volume and tell us nothing. Failures
    # mean port exhaustion, seen as image pulls timing out on new nodes.
    filter = "ERRORS_ONLY"
  }
}
