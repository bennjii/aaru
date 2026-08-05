# Installs the routers-realtime chart as a single release.
#
# One release, and it has to be one: the orchestrator fleet is a single
# StatefulSet dividing the vehicle partition space between its ordinals, so
# rendering it more than once would hand the same partitions to two owners.
# The matchers could be split by geography, but putting the two halves of one
# pipeline behind two apply paths buys nothing — they are deployed together and
# versioned together.
#
# Everything geographic ends at the matcher: a shard is a subject
# (`events.match.<shard>`) served by a queue group, and the orchestrator
# reaches it by hashing the event's position with the precision compiled into
# the binary. That is the whole coupling between the two halves.

locals {
  redis_value = join(",", var.valkey_client_mode == "pooled-hash" ? var.valkey_urls : [var.valkey_urls[0]])

  # The standard OTel sampler pair, rendered through each service's `env` map
  # rather than a chart field: the chart passes those maps verbatim into the
  # container, so a knob that is only an environment variable needs no template
  # support to become deployable.
  sampler_env = var.telemetry_sample_ratio == "" ? {} : {
    OTEL_TRACES_SAMPLER     = "parentbased_traceidratio"
    OTEL_TRACES_SAMPLER_ARG = var.telemetry_sample_ratio
  }

  values = {
    shards         = var.shards
    shardPrecision = var.shard_precision
    streams        = var.streams

    infra = {
      # One NATS cluster and one Valkey fleet, neither partitioned
      # geographically: the keyspace is per vehicle, and vehicles cross shards.
      nats   = { url = var.nats_url }
      valkey = { url = local.redis_value }
      otlp   = { url = var.otlp_url }
    }

    image = {
      registry = var.image_registry
    }

    imagePullSecrets = var.image_pull_secrets

    serviceAccount = {
      create = var.service_account_create
      name   = var.service_account_name
      annotations = var.gcp_service_account_email == "" ? {} : {
        "iam.gke.io/gcp-service-account" = var.gcp_service_account_email
      }
    }

    shardCache = merge(
      { mode = var.shard_cache_mode },
      var.shard_cache_mode == "gcsfuse" ? {
        gcs = {
          bucket       = var.shard_cache_bucket
          mountOptions = var.shard_cache_mount_options
        }
      } : {},
      var.shard_cache_mode == "hostPath" ? { hostPath = var.shard_cache_host_path } : {},
      var.shard_cache_mode == "pvc" ? { pvc = { claimName = var.shard_cache_pvc_name } } : {},
    )

    matcher = {
      image = {
        repository = "routers-matcher"
        tag        = var.image_tag
        pullPolicy = var.image_pull_policy
      }
      rustLog  = var.log_level
      replicas = var.matcher_replicas

      autoscaling = {
        enabled                        = var.matcher_autoscaling
        minReplicas                    = var.matcher_replicas
        maxReplicas                    = var.matcher_replicas_max
        targetCPUUtilizationPercentage = var.matcher_target_cpu_utilization
      }

      # The matcher fans each boundary weighing across rayon, so the worker
      # count is what uses the node's cores.
      env = merge(
        { WORKERS = tostring(var.profile.matcher_workers) },
        local.sampler_env,
        var.matcher_env,
      )

      nodeSelector = var.matcher_node_selector
      tolerations  = var.matcher_tolerations

      resources = {
        # No CPU limit. A CFS cap freezes every worker and the rayon pool
        # together in 100ms throttle windows. The request still guarantees
        # scheduling; the memory limit still bounds the pod.
        requests = {
          cpu    = "${var.profile.matcher_cpu_millis}m"
          memory = "${var.matcher_memory_mib}Mi"
        }
        limits = {
          memory = "${var.matcher_memory_mib}Mi"
        }
      }
    }

    orchestrator = {
      image = {
        repository = "routers-orchestrator"
        tag        = var.image_tag
        pullPolicy = var.image_pull_policy
      }
      rustLog = var.log_level

      # Not a replica count in the usual sense: the chart feeds this to the
      # binary as FLEET alongside the pod's ordinal, and the two derive which
      # partitions this pod owns.
      replicas = var.fleet_size

      # A worker holds its vehicle's lane across a whole solve round trip, so
      # this is the pod's in-flight solve bound.
      env = merge(
        { WORKERS = tostring(var.profile.orchestrator_workers) },
        local.sampler_env,
        var.orchestrator_env,
      )

      nodeSelector = var.pipeline_node_selector
      tolerations  = var.pipeline_tolerations

      resources = {
        requests = {
          cpu    = "${var.profile.orchestrator_cpu_millis}m"
          memory = "${var.profile.orchestrator_memory_mib}Mi"
        }
        limits = {
          # Headroom for the per-vehicle trip and history lanes, which grow
          # over a run. A reclaim stall at the limit looks like latency.
          memory = "${var.profile.orchestrator_memory_mib}Mi"
        }
      }
    }

    grafanaDashboard = { enabled = var.grafana_dashboard_enabled }
  }
}

resource "helm_release" "realtime" {
  name             = var.release_name
  chart            = var.chart_path
  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait         = var.wait_for_rollout
  timeout      = var.release_timeout
  max_history  = var.max_history
  atomic       = false
  reset_values = true

  values = [yamlencode(local.values)]
}

# Configuration that would otherwise fail at runtime, not at plan time.
check "shard_cache_source_is_configured" {
  assert {
    condition = (
      var.shard_cache_mode != "gcsfuse" || var.shard_cache_bucket != ""
    )
    error_message = "shard_cache_mode is 'gcsfuse' but shard_cache_bucket is empty; matchers would start with an empty /shards and match nothing."
  }

  assert {
    condition = (
      var.shard_cache_mode != "hostPath" || var.shard_cache_host_path != ""
    )
    error_message = "shard_cache_mode is 'hostPath' but shard_cache_host_path is empty."
  }

  assert {
    condition = (
      var.shard_cache_mode != "pvc" || var.shard_cache_pvc_name != ""
    )
    error_message = "shard_cache_mode is 'pvc' but shard_cache_pvc_name is empty."
  }
}

check "gcsfuse_can_authenticate" {
  assert {
    condition = (
      var.shard_cache_mode != "gcsfuse" ||
      (var.service_account_name != "" && var.gcp_service_account_email != "")
    )
    error_message = "gcsfuse needs Workload Identity: set service_account_name and gcp_service_account_email, or the CSI driver cannot read the bucket."
  }
}

check "shards_match_the_declared_precision" {
  assert {
    condition     = alltrue([for s in var.shards : length(s) == var.shard_precision])
    error_message = "Every shard must be exactly shard_precision characters. A shorter or longer one makes the matcher load a different extent from the subject it serves, and the orchestrator's requests reach nothing."
  }
}

check "the_release_fits_a_kubernetes_secret" {
  assert {
    condition     = length(var.shards) <= var.max_shards_per_release
    error_message = "This release renders ${length(var.shards)} shards, past the ${var.max_shards_per_release} guard. Every manifest lives in one Secret and a Secret holds 1 MiB; raise shard_precision only with the shard cache regenerated to match, or raise the guard if the release still applies."
  }
}

check "the_fleet_divides_the_partition_space" {
  assert {
    # 1024 is `partition::PARTITIONS`. Pods take contiguous ordinal blocks and
    # the last takes the remainder, so a non-divisor leaves one pod owning more
    # partitions — and so more vehicles — than the rest.
    condition     = 1024 % var.fleet_size == 0
    error_message = "fleet_size ${var.fleet_size} does not divide the 1024 vehicle partitions. The binary gives the last pod the remainder, making it the hot one; use a power of two."
  }
}

check "matcher_autoscaling_has_room" {
  assert {
    condition     = !var.matcher_autoscaling || var.matcher_replicas_max >= var.matcher_replicas
    error_message = "matcher_replicas_max (${var.matcher_replicas_max}) is below the floor of ${var.matcher_replicas}, so the HPA would scale a shard down out of its own capacity."
  }
}

check "image_tag_is_reproducible" {
  assert {
    condition     = var.image_tag != "latest" || var.image_pull_policy == "Never"
    error_message = "image_tag is 'latest' on a cluster that pulls images, so a rollout is not reproducible and a rollback has nothing to return to. Pin a digest or an immutable tag."
  }
}
