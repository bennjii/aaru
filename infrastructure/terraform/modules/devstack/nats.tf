# One NATS cluster for the whole deployment, carrying both halves of the
# pipeline: core request/reply for the solves, and JetStream for durable
# ingest.
#
# A server forwards a core message only to routes that have registered matching
# subscription interest, and never more than one hop, so a cluster already
# partitions by subject. An earlier revision gave each geographic cell its own
# cluster and paid a three-server availability floor per cell for locality that
# interest routing was giving away.
#
# JetStream is no longer optional, and not merely for at-least-once delivery:
# the orchestrator's ingest is a work-queue stream per block of vehicle
# partitions with one filtered durable consumer per partition, and a revision —
# the total order competing solves resolve by — is a stream sequence. Nothing
# in the pipeline functions without it, which is also why the server floor is
# now three: JetStream elects a leader per stream, and at one replica a lost
# server takes its streams' unacked backlog with it.

locals {
  nats_resources = {
    requests = {
      cpu    = "${var.nats_cpu_millis}m"
      memory = "${var.nats_memory_mib}Mi"
    }
    # No CPU limit. A CFS cap would throttle the routing loop in 100ms windows
    # and stall every in-flight message behind the freeze — now including the
    # raft heartbeats that keep stream leadership stable.
    limits = {
      memory = "${var.nats_memory_mib}Mi"
    }
  }

  nats_pod_spec = merge(
    length(var.node_selector) > 0 ? { nodeSelector = var.node_selector } : {},
    length(var.tolerations) > 0 ? { tolerations = var.tolerations } : {},
  )
}

resource "helm_release" "nats" {
  name       = "nats"
  repository = "https://nats-io.github.io/k8s/helm/charts/"
  chart      = "nats"
  version    = var.chart_versions.nats

  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait    = var.wait_for_rollout
  timeout = var.release_timeout

  values = [yamlencode({
    # Fixes the Service name, so the client URL is derivable rather than
    # dependent on the release name the chart happens to build.
    fullnameOverride = "nats"

    config = {
      cluster = {
        enabled  = true
        replicas = var.nats_replicas
      }

      jetstream = {
        enabled = true

        # File, not memory. A work queue holds the only copy of an event until
        # it is acked, so the store is the ingest buffer itself rather than a
        # cache: memory storage would discard the backlog on restart, which is
        # exactly the case the durability exists for.
        #
        # Sized per server by the capacity model from the retained backlog and
        # the matched stream's retention. The matched stream dominates it —
        # emissions carry the whole cut trip, so its byte rate is a multiple of
        # ingest — and it is a PVC per pod, so this is the number to watch when
        # the target moves.
        fileStore = {
          enabled = true
          pvc = merge(
            {
              enabled = true
              size    = "${var.jetstream_file_store_gib}Gi"
            },
            # Left to the cluster default when unset, which is pd-balanced on
            # GKE. Worth setting to an SSD class if the write ceiling turns out
            # to be disk rather than CPU.
            var.jetstream_storage_class == "" ? {} : { storageClassName = var.jetstream_storage_class },
          )
        }
      }
    }

    container = {
      resources = local.nats_resources
    }

    podTemplate = merge(
      {
        # Spread servers across nodes, or the availability floor buys nothing.
        topologySpreadConstraints = {
          "kubernetes.io/hostname" = {
            maxSkew           = 1
            whenUnsatisfiable = "ScheduleAnyway"
          }
        }
      },
      length(local.nats_pod_spec) > 0 ? { merge = { spec = local.nats_pod_spec } } : {},
    )

    # The pipeline has no metrics of its own, so NATS' own series are the only
    # direct view of queue depth and slow consumers.
    promExporter = {
      enabled = true
      port    = 7777
      podMonitor = {
        enabled = true
        merge = {
          spec = {
            podMetricsEndpoints = [{
              port     = "prom-metrics"
              interval = "10s"
            }]
          }
        }
      }
    }

    natsBox = {
      enabled = false
    }
  })]
}
