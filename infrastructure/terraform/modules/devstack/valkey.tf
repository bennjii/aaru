# The Valkey fleet: N independent primaries, each its own release.
#
# Not a Valkey Cluster, and not partitioned geographically. Clients place a
# vehicle by rendezvous hash over the URL list this module outputs, so there is
# no slot map, no MOVED handling and no `cluster-async` feature. A primary's
# identity is its URL, not its index, so `valkey_urls` order does not matter
# and a resize moves about 1/N of vehicles.

locals {
  # Zero-padded so both the release names and the URL list sort in index order.
  valkey_indices = { for i in range(var.valkey_primaries) : format("%03d", i) => i }

  valkey_resources = {
    requests = {
      cpu    = "${var.valkey_cpu_millis}m"
      memory = "${var.valkey_memory_mib}Mi"
    }
    # No CPU limit, deliberately. Command execution is single-threaded, so a
    # CFS cap freezes the loop in ~100ms windows and every in-flight command
    # queues behind it: measured as a 50-100ms stall tail on reads while
    # execution itself stayed at ~7us per call.
    limits = {
      memory = "${var.valkey_memory_mib}Mi"
    }
  }

  valkey_image = {
    for k, v in {
      registry   = var.valkey_image.registry
      repository = var.valkey_image.repository
      tag        = var.valkey_image.tag
    } : k => v if v != ""
  }

  valkey_scheduling = merge(
    length(var.node_selector) > 0 ? { nodeSelector = var.node_selector } : {},
    length(var.tolerations) > 0 ? { tolerations = var.tolerations } : {},
  )

  # Replication is for failover only, so zero replicas is legitimate.
  valkey_architecture = var.valkey_replicas_per_primary > 0 ? "replication" : "standalone"

  # io-threads is the vertical lever: threads parse and write while command
  # execution stays on the main thread. Passed as a flag, not through
  # `configuration`, which replaces the chart's whole config file rather than
  # adding to it and would silently drop every default the chart sets.
  valkey_flags = ["--io-threads", tostring(var.valkey_io_threads)]
}

resource "helm_release" "valkey" {
  for_each = local.valkey_indices

  name    = "valkey-${each.key}"
  chart   = "oci://registry-1.docker.io/bitnamicharts/valkey"
  version = var.chart_versions.valkey

  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait    = var.wait_for_rollout
  timeout = var.release_timeout

  values = [yamlencode(merge(
    {
      fullnameOverride = "valkey-${each.key}"

      # No auth. The keyspace is reachable only from inside the cluster, and a
      # password costs a handshake per reconnect on the hot path for no
      # boundary the network does not already enforce.
      auth = {
        enabled = false
      }

      architecture = local.valkey_architecture

      primary = merge({
        resources   = local.valkey_resources
        extraFlags  = local.valkey_flags
        persistence = { enabled = var.valkey_persistence }
      }, local.valkey_scheduling)

      replica = merge({
        replicaCount = var.valkey_replicas_per_primary
        resources    = local.valkey_resources
        extraFlags   = local.valkey_flags
        persistence  = { enabled = var.valkey_persistence }
      }, local.valkey_scheduling)

      metrics = {
        enabled = true
        serviceMonitor = {
          enabled = true
        }
      }
    },
    length(local.valkey_image) > 0 ? { image = local.valkey_image } : {},
  ))]
}

check "valkey_client_can_address_the_fleet" {
  assert {
    condition = var.valkey_client_mode == "pooled-hash" || var.valkey_primaries == 1
    error_message = join(" ", [
      "valkey_client_mode is 'single' but the fleet has ${var.valkey_primaries} primaries,",
      "so clients would reach only the first URL and the rest would sit idle.",
      "Set 'pooled-hash', or set valkey_primaries to 1.",
    ])
  }
}

check "valkey_image_is_pinned" {
  assert {
    condition = var.valkey_image.tag != ""
    error_message = join(" ", [
      "valkey_image.tag is unset, so the Bitnami chart default applies.",
      "That default is `registry-1.docker.io/bitnami/valkey:latest`, and its metrics",
      "sidecar is `bitnami/redis-exporter:latest`: floating tags, from a catalogue",
      "Bitnami moved to `bitnamilegacy/*` during 2025.",
      "Mirror a pinned digest into Artifact Registry and set valkey_image.",
    ])
  }
}
