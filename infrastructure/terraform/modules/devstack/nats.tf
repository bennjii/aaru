# One core NATS cluster for the whole deployment.
#
# A server forwards a message only to routes that have registered matching
# subscription interest, and never more than one hop, so a cluster already
# partitions by subject. An earlier revision gave each cell its own cluster and
# paid a three-server availability floor per cell for locality that interest
# routing was giving away.

locals {
  nats_resources = {
    requests = {
      cpu    = "${var.nats_cpu_millis}m"
      memory = "${var.nats_memory_mib}Mi"
    }
    # No CPU limit. A CFS cap would throttle the routing loop in 100ms windows
    # and stall every in-flight message behind the freeze.
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

      # Off deliberately. JetStream would buy at-least-once delivery, but the
      # official benchmarks put a single stream well under core NATS on the same
      # hardware, and the durability it offers is already held in Valkey — which
      # is what a restarted matcher resumes from.
      jetstream = {
        enabled = false
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
