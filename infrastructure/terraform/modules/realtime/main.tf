# Installs the routers-realtime chart, one release per cell, plus one shared
# release for what must exist exactly once: the Grafana dashboard ConfigMap
# and the global historian. The shared release declares no shards, so it
# renders no matchers or orchestrators. See ../../ARCHITECTURE.md.

locals {
  all_shards = flatten([for c in var.cell_plan : c.shards])

  redis_value = join(",", var.valkey_client_mode == "pooled-hash" ? var.valkey_urls : [var.valkey_urls[0]])

  # Shared by every release. Per-cell values layer on top.
  common_values = {
    shardPrecision = var.shard_precision
    cellPrecision  = var.cell_precision
    subjectPrefix  = var.subject_prefix

    infra = {
      otlp = { url = var.otlp_url }
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

    serviceAccount = {
      create = var.service_account_create
      name   = var.service_account_name
      annotations = var.gcp_service_account_email == "" ? {} : {
        "iam.gke.io/gcp-service-account" = var.gcp_service_account_email
      }
    }

    imagePullSecrets = var.image_pull_secrets

    telemetry = {
      sampleRatio = var.telemetry_sample_ratio
    }

    matcher = {
      image = {
        registry   = var.image_registry
        repository = "routers-matcher"
        tag        = var.image_tag
        pullPolicy = var.image_pull_policy
      }
      rustLog = var.log_level
      # The matcher fans each boundary weighing across rayon, so the worker
      # count is what uses the node's cores.
      env               = merge({ WORKERS = tostring(var.profile.matcher_workers) }, var.matcher_env)
      nodeSelector      = var.matcher_node_selector
      tolerations       = var.matcher_tolerations
      priorityClassName = var.priority_class_name
      resources = {
        # No CPU limit. A CFS cap freezes every worker and the rayon pool
        # together in 100ms throttle windows. The request still guarantees
        # scheduling; the memory limit still bounds the pod.
        requests = {
          cpu    = "${var.profile.matcher_cpu_millis}m"
          memory = "${var.profile.matcher_memory_mib}Mi"
        }
        limits = {
          memory = "${var.profile.matcher_memory_mib}Mi"
        }
      }
    }

    orchestrator = {
      image = {
        registry   = var.image_registry
        repository = "routers-orchestrator"
        tag        = var.image_tag
        pullPolicy = var.image_pull_policy
      }
      rustLog = var.log_level
      # Each vehicle is pinned to one worker, so per-vehicle ordering holds at
      # any count. The count overlaps the per-event Valkey fetch, which is the
      # throughput bound.
      env               = merge({ WORKERS = tostring(var.profile.orchestrator_workers) }, var.orchestrator_env)
      nodeSelector      = var.pipeline_node_selector
      tolerations       = var.pipeline_tolerations
      priorityClassName = var.priority_class_name
      resources = {
        requests = {
          cpu    = "${var.profile.orchestrator_cpu_millis}m"
          memory = "${var.profile.orchestrator_memory_mib}Mi"
        }
        limits = {
          # Headroom for the per-vehicle trip map, which grows over a run. A
          # reclaim stall at the limit looks like latency.
          memory = "${var.profile.orchestrator_memory_mib}Mi"
        }
      }
    }

    historian = {
      image = {
        registry   = var.image_registry
        repository = "routers-historian"
        tag        = var.image_tag
        pullPolicy = var.image_pull_policy
      }
      rustLog           = var.log_level
      mode              = var.historian_mode
      env               = var.historian_env
      nodeSelector      = var.pipeline_node_selector
      tolerations       = var.pipeline_tolerations
      priorityClassName = var.priority_class_name
      resources = {
        requests = {
          cpu    = "${var.profile.historian_cpu_millis}m"
          memory = "${var.profile.historian_memory_mib}Mi"
        }
        limits = {
          memory = "${var.profile.historian_memory_mib}Mi"
        }
      }
    }
  }
}

resource "helm_release" "cell" {
  for_each = var.cell_plan

  name             = "${var.release_name}-${each.key}"
  chart            = var.chart_path
  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait         = var.wait_for_rollout
  timeout      = var.release_timeout
  max_history  = var.max_history
  atomic       = false
  reset_values = true

  values = [yamlencode(merge(local.common_values, {
    shards = each.value.shards
    cell   = each.key

    infra = merge(local.common_values.infra, {
      # One NATS cluster and one Valkey fleet for every cell. Neither is
      # partitioned geographically. See ../../ARCHITECTURE.md.
      nats   = { url = var.nats_url }
      valkey = { url = local.redis_value }
    })

    historian = merge(local.common_values.historian, {
      enabled    = contains(["per-cell", "per-shard"], var.historian_mode)
      replicas   = try(var.historian_replicas_by_cell[each.key], 1)
      queueGroup = var.historian_queue_group
    })

    # The ConfigMap has one fixed name, so rendering it per cell would make the
    # releases fight over the same object.
    grafanaDashboard = { enabled = false }
  }))]
}

resource "helm_release" "shared" {
  name             = "${var.release_name}-shared"
  chart            = var.chart_path
  namespace        = var.namespace
  create_namespace = var.create_namespace

  wait         = var.wait_for_rollout
  timeout      = var.release_timeout
  max_history  = var.max_history
  atomic       = false
  reset_values = true

  values = [yamlencode(merge(local.common_values, {
    shards = []

    # The chart requires both URLs even when nothing here consumes them.
    infra = merge(local.common_values.infra, {
      nats   = { url = var.nats_url }
      valkey = { url = local.redis_value }
    })

    historian = merge(local.common_values.historian, {
      enabled = var.historian_mode == "global"
      # One replica, always. A second one writes every event again.
      replicas   = 1
      queueGroup = false
    })

    grafanaDashboard = { enabled = var.grafana_dashboard_enabled }
  }))]
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

check "historian_replicas_need_a_queue_group" {
  assert {
    condition = (
      var.historian_queue_group ||
      alltrue([for cell, n in var.historian_replicas_by_cell : n <= 1])
    )
    error_message = "A cell is configured with more than one historian replica but historian_queue_group is false. Every replica would receive every event and write it again. Set historian_queue_group, and make sure the image is new enough to read QUEUE_GROUP."
  }
}

check "cells_fit_a_helm_release" {
  assert {
    condition = alltrue([
      for cell, plan in var.cell_plan : length(plan.shards) <= var.max_shards_per_cell
    ])
    error_message = "A cell holds more than ${var.max_shards_per_cell} shards, so its release approaches Helm's 1 MiB Secret limit. Raise cell_precision to split it."
  }
}

check "image_tag_is_reproducible" {
  assert {
    condition     = var.image_tag != "latest" || var.image_pull_policy == "Never"
    error_message = "image_tag is 'latest' on a cluster that pulls images. With one replica per shard a bad image takes that shard offline with nothing to compare against, so pin a digest or an immutable tag."
  }
}
