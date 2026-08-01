# The span-to-metric bridge, and Prometheus to scrape it.
#
# The routers binaries hold no metric registry and expose no HTTP endpoint, so
# a span is their only telemetry primitive. The spanmetrics connector turns
# spans into duration histograms and call counters, scraped from :8889. Break
# this and the pipeline goes blind, not just unmonitored. Traces are
# aggregated, not stored; add a Tempo exporter for browsing.

locals {
  collector_scheduling = merge(
    length(var.observability_node_selector) > 0 ? { nodeSelector = var.observability_node_selector } : {},
    length(var.observability_tolerations) > 0 ? { tolerations = var.observability_tolerations } : {},
  )
}

resource "helm_release" "collector" {
  name       = "otel-collector"
  repository = "https://open-telemetry.github.io/opentelemetry-helm-charts"
  chart      = "opentelemetry-collector"
  version    = var.chart_versions.collector

  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait    = var.wait_for_rollout
  timeout = var.release_timeout

  values = [yamlencode(merge({
    mode         = "deployment"
    replicaCount = var.collector_replicas

    # spanmetrics lives in the contrib distribution only.
    image = {
      repository = "otel/opentelemetry-collector-contrib"
    }
    command = {
      name = "otelcol-contrib"
    }

    resources = {
      requests = {
        cpu    = "${var.collector_cpu_millis}m"
        memory = "${var.collector_memory_mib}Mi"
      }
      # No CPU limit: the work arrives in bursts that follow the pipeline's
      # own, and throttling here drops spans when they matter most.
      limits = {
        memory = "${var.collector_memory_mib}Mi"
      }
    }

    ports = {
      spanmetrics = {
        enabled       = true
        containerPort = 8889
        servicePort   = 8889
        protocol      = "TCP"
      }
      # accepted/refused/dropped: the meta-signal for when the routers series
      # go quiet.
      metrics = {
        enabled = true
      }
      # Only OTLP is spoken here.
      jaeger-compact = { enabled = false }
      jaeger-thrift  = { enabled = false }
      jaeger-grpc    = { enabled = false }
      zipkin         = { enabled = false }
    }

    serviceMonitor = {
      enabled = var.observability_enabled
      metricsEndpoints = [
        { port = "metrics" },
        { port = "spanmetrics", interval = var.metrics_flush_interval },
      ]
    }

    config = {
      receivers = {
        otlp = {
          protocols = {
            grpc = { endpoint = "0.0.0.0:4317" }
            http = { endpoint = "0.0.0.0:4318" }
          }
        }
      }

      connectors = {
        spanmetrics = {
          namespace              = "routers"
          metrics_flush_interval = var.metrics_flush_interval
          histogram = {
            unit = "ms"
            explicit = {
              # Sub-second buckets resolve the compute spans. The tail to 120s
              # is for the wait spans: a backlogged run parks messages for tens
              # of seconds, and a 5s cap would censor that.
              buckets = [
                "1ms", "2ms", "5ms", "10ms", "25ms", "50ms", "100ms", "250ms",
                "500ms", "1s", "2500ms", "5s", "10s", "30s", "60s", "120s",
              ]
            }
          }
          dimensions = [for d in var.span_metrics_dimensions : { name = d }]
        }
      }

      exporters = {
        prometheus = {
          endpoint = "0.0.0.0:8889"
        }
      }

      service = {
        pipelines = {
          traces = {
            receivers = ["otlp"]
            exporters = ["spanmetrics"]
          }
          metrics = {
            receivers = ["spanmetrics"]
            exporters = ["prometheus"]
          }
          logs = null
        }
      }
    }
  }, local.collector_scheduling))]
}

resource "helm_release" "prometheus" {
  count = var.observability_enabled ? 1 : 0

  name       = "kube-prometheus-stack"
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "kube-prometheus-stack"
  version    = var.chart_versions.prometheus

  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait    = var.wait_for_rollout
  timeout = var.release_timeout

  values = [yamlencode({
    grafana = merge(
      {
        fullnameOverride = "grafana"

        # Discover dashboard ConfigMaps in every namespace, so the routers
        # chart ships its own.
        sidecar = {
          dashboards = {
            searchNamespace = "ALL"
          }
        }

        "grafana.ini" = merge(
          {
            users = { allow_sign_up = false }
            dashboards = {
              # The sidecar writes dashboards into /tmp/dashboards, keyed by
              # their ConfigMap data key.
              default_home_dashboard_path = "/tmp/dashboards/routers-realtime.json"
            }
          },
          # Developer machines only: anyone who reaches the service becomes an
          # administrator.
          var.grafana_anonymous_admin ? {
            "auth.anonymous" = {
              enabled  = true
              org_role = "Admin"
            }
            auth = {
              disable_login_form = true
            }
          } : {},
        )
      },
      local.collector_scheduling,
    )

    prometheus = {
      prometheusSpec = merge(
        {
          # Discover ServiceMonitors and PodMonitors from any namespace with
          # any labels, so routers releases need not match this release's
          # Helm labels.
          serviceMonitorSelectorNilUsesHelmValues = false
          serviceMonitorSelector                  = {}
          serviceMonitorNamespaceSelector         = {}
          podMonitorSelectorNilUsesHelmValues     = false
          podMonitorSelector                      = {}
          podMonitorNamespaceSelector             = {}
        },
        local.collector_scheduling,
      )
    }

    alertmanager = {
      alertmanagerSpec = local.collector_scheduling
    }
  })]
}
