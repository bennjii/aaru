# The GKE cluster. Standard, not Autopilot: the pipeline needs custom node
# pools, local NVMe on the matcher pool, and pods with no CPU limit. Autopilot
# rejects all three — it forces a CPU limit equal to the request, and a CFS cap
# on a matcher freezes its rayon pool in 100ms throttle windows.

resource "google_container_cluster" "cluster" {
  project  = var.project_id
  name     = var.cluster_name
  location = var.region

  deletion_protection = var.deletion_protection

  network    = google_compute_network.vpc.id
  subnetwork = google_compute_subnetwork.nodes.id

  # A regional cluster replicates the control plane across the region's zones
  # and spreads nodes across them, so a zonal outage costs a fraction of the
  # matchers and a fraction of the orchestrator fleet rather than either
  # outright. Upgrades and API calls are slower.
  #
  # The default pool is created and then removed, because its node config
  # cannot be controlled. initial_node_count is required even so.
  remove_default_node_pool = true
  initial_node_count       = 1

  # VPC-native. Pod IPs are real VPC addresses out of the secondary range, so
  # they are routable and the pod range arithmetic in main.tf applies.
  ip_allocation_policy {
    cluster_secondary_range_name  = local.pods_range_name
    services_secondary_range_name = local.services_range_name
  }

  default_max_pods_per_node = var.max_pods_per_node

  # Dataplane V2 is eBPF-based (Cilium). The iptables dataplane degrades as
  # service and endpoint counts grow, and this cluster carries a Deployment per
  # shard plus the orchestrator fleet. It cannot be turned off without
  # rebuilding.
  datapath_provider = "ADVANCED_DATAPATH"

  private_cluster_config {
    # No public IPs on nodes. Egress goes through Cloud NAT.
    enable_private_nodes = true

    # The endpoint stays public so CI and operators reach it without a bastion
    # or a VPN. Only master_authorized_cidrs can connect. A private endpoint is
    # tighter, but then every deploy must run from inside the VPC.
    enable_private_endpoint = false
    master_ipv4_cidr_block  = var.master_ipv4_cidr_block
  }

  master_authorized_networks_config {
    # An empty list is a closed door, not an open one: an environment that has
    # not declared its operators is unreachable from outside Google Cloud.
    dynamic "cidr_blocks" {
      for_each = var.master_authorized_cidrs
      content {
        cidr_block   = cidr_blocks.value.cidr_block
        display_name = cidr_blocks.value.display_name
      }
    }
  }

  # Workload Identity replaces service account keys: the shard cache account
  # binds to a Kubernetes one, so there is no JSON key to leak.
  workload_identity_config {
    workload_pool = "${var.project_id}.svc.id.goog"
  }

  addons_config {
    # Matcher pods mount the shard cache bucket read-only through this driver,
    # so shard files need no image baking or hostPath sync.
    gcs_fuse_csi_driver_config {
      enabled = true
    }

    # Nothing is served over HTTP, so there is no Ingress to run it for.
    http_load_balancing {
      disabled = true
    }

    # Node pools declare their own bounds, sized by the capacity module.
    # Vertical pod autoscaling would fight those numbers.
    horizontal_pod_autoscaling {
      disabled = false
    }
  }

  # Node pool autoscaling only. Node auto-provisioning stays off: a machine
  # type chosen at runtime is a machine type nobody sized.
  #
  # OPTIMIZE_UTILIZATION removes underused nodes sooner than BALANCED. That was
  # safe when every pod was pinned to a shard for its lifetime; matchers now
  # carry an HPA, so scale-down does follow traffic. It stays because the
  # eviction it risks is a matcher replica — one member of a shard's queue
  # group, whose requests the others take — and never an orchestrator, which
  # has no autoscaler and owns its partitions alone.
  cluster_autoscaling {
    enabled             = false
    autoscaling_profile = "OPTIMIZE_UTILIZATION"
  }

  release_channel {
    channel = var.release_channel
  }

  logging_config {
    # System components and the control plane only. The services already export
    # logs and spans through the collector, and Cloud Logging bills per GiB.
    enable_components = [
      "SYSTEM_COMPONENTS",
      "APISERVER",
      "CONTROLLER_MANAGER",
      "SCHEDULER",
    ]
  }

  monitoring_config {
    enable_components = [
      "SYSTEM_COMPONENTS",
      "APISERVER",
      "CONTROLLER_MANAGER",
      "SCHEDULER",
      # Per-container CPU and memory: how to tell a saturated matcher from a
      # starved one.
      "KUBELET",
      "CADVISOR",
    ]

    # Managed Prometheus scrapes the cluster with no server to operate. It is
    # the managed equivalent of kube-prometheus-stack in infrastructure/dev.
    managed_prometheus {
      enabled = true
    }
  }

  master_auth {
    # Legacy client certificate auth issues a credential that cannot be revoked
    # and does not expire. Workload Identity and IAM cover every access path.
    client_certificate_config {
      issue_client_certificate = false
    }
  }

  resource_labels = merge(var.labels, {
    env       = var.env
    component = "routers-platform"
  })

  lifecycle {
    ignore_changes = [
      # GKE and other controllers write their own labels onto the cluster.
      # Without this, every plan after an addon update shows a label diff.
      resource_labels["goog-composer-environment"],
    ]
  }

  depends_on = [
    # Nodes that boot before NAT exists cannot pull images, and pool creation
    # then times out.
    google_compute_router_nat.nat,
  ]
}
