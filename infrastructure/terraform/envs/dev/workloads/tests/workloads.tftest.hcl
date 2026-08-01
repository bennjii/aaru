# Wiring test for the workloads root. Mock providers stand in for GCP, Helm and
# Kubernetes, so this runs with no cluster: it checks that the sizing model
# reaches the releases intact, not that Helm installs anything.

mock_provider "google" {
  mock_data "google_container_cluster" {
    defaults = {
      endpoint = "10.0.0.1"
      # Every field of the block is required, even the legacy certificate ones
      # the cluster disables.
      master_auth = [{
        cluster_ca_certificate    = "bW9jaw=="
        client_certificate        = ""
        client_key                = ""
        client_certificate_config = []
      }]
    }
  }
}

mock_provider "helm" {}
mock_provider "kubernetes" {}

variables {
  project_id                        = "routers-test"
  image_registry                    = "australia-southeast1-docker.pkg.dev/routers-test/routers"
  image_tag                         = "sha256-abc123"
  shard_bucket                      = "routers-test-shards"
  shard_cache_service_account_email = "dev-shard-cache@routers-test.iam.gserviceaccount.com"

  # The devstack refuses an unpinned Valkey image, because the Bitnami chart
  # default is a floating tag from a relocated catalogue.
  valkey_image = {
    registry   = "australia-southeast1-docker.pkg.dev"
    repository = "routers-test/routers/valkey"
    tag        = "8.1.1-debian-12-r0"
  }
}

run "capacity_reaches_the_releases_intact" {
  command = plan

  # One release per cell, plus the shared one carrying the dashboard.
  assert {
    condition     = module.realtime.cell_count == 2
    error_message = "Expected 2 cell releases, got ${module.realtime.cell_count}."
  }

  assert {
    condition     = length(module.realtime.release_names) == 3
    error_message = "Expected 2 cell releases and 1 shared, got ${join(", ", module.realtime.release_names)}."
  }

  assert {
    condition     = module.realtime.shard_count == 6
    error_message = "Expected 6 shards deployed, got ${module.realtime.shard_count}."
  }
}

# The whole point of the two-phase subject: cell and shard as separate tokens,
# so a wildcard can address either level.
run "subjects_are_split_across_two_tokens" {
  command = plan

  assert {
    condition     = module.realtime.subjects["r3gq"].position == "events.position.r3.gq"
    error_message = "Expected events.position.r3.gq, got ${module.realtime.subjects["r3gq"].position}."
  }

  assert {
    condition = alltrue([
      for shard, s in module.realtime.subjects : "${s.cell}${s.suffix}" == shard
    ])
    error_message = "A shard's cell and suffix do not reconstruct its geohash."
  }
}

# Every workload reaches the whole Valkey fleet, because a vehicle's history has
# to be found wherever it drives.
run "workloads_receive_the_whole_valkey_fleet" {
  command = plan

  assert {
    condition     = length(split(",", module.realtime.redis_value)) == module.capacity.valkey.primaries
    error_message = "REDIS should list all ${module.capacity.valkey.primaries} primaries, got ${module.realtime.redis_value}."
  }
}

# One broker for the whole deployment. Cells no longer imply a cluster each.
run "every_cell_shares_one_nats_cluster" {
  command = plan

  assert {
    condition     = module.devstack.nats_url == "nats://nats.routers-dev.svc.cluster.local:4222"
    error_message = "Expected the single cluster URL, got ${module.devstack.nats_url}."
  }

  assert {
    condition     = module.devstack.totals.nats_servers == module.capacity.nats.replicas_total
    error_message = "The devstack must install the server count the model sized."
  }
}

# Pod totals are the cross-check between the two halves: if the chart renders
# something other than what was sized, these disagree.
run "pod_totals_agree_with_the_model" {
  command = plan

  assert {
    condition = module.realtime.expected_pod_count == (
      module.capacity.replicas.matcher
      + module.capacity.replicas.orchestrator
      + module.capacity.replicas.historian
    )
    error_message = "Chart pods (${module.realtime.expected_pod_count}) disagree with the model."
  }
}
