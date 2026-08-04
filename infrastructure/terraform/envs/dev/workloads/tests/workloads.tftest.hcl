# Wiring test for the workloads root. Mock providers stand in for GCP, Helm and
# Kubernetes, so this runs with no cluster: it checks that the sizing model
# reaches the release intact, not that Helm installs anything.

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

# Every run below deploys the shipped 800k target, which the current binaries
# cannot fully absorb: `ingest::MATCHED_STREAM` is one stream, so one raft
# leader carries every emission. `tofu test` treats a failing check as an error
# (a plan only warns), so the two checks that report it are declared expected.
#
# This is deliberately not hidden behind a tolerant condition. When the matched
# stream is partitioned in `ingest.rs` and `matched_streams` is raised, these
# expectations start failing — which is the prompt to delete them.


# One release, and only one. The orchestrator fleet is a single StatefulSet
# over the vehicle partition space, so a second release rendering it would hand
# the same partitions to two owners.
run "the_whole_deployment_is_one_release" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = length(module.realtime.release_names) == 1
    error_message = "Expected exactly one release, got ${join(", ", module.realtime.release_names)}."
  }

  assert {
    condition     = module.realtime.shard_count == 6
    error_message = "Expected 6 shards deployed, got ${module.realtime.shard_count}."
  }
}

# The two axes must arrive independently: geography sets the matcher count, the
# partition space sets the fleet, and neither is derived from the other.
run "both_axes_reach_the_release" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = module.realtime.expected_pod_count.orchestrators == module.capacity.fleet.size
    error_message = "The fleet size must reach the release unchanged: ${module.realtime.expected_pod_count.orchestrators} against ${module.capacity.fleet.size}."
  }

  assert {
    condition     = module.realtime.expected_pod_count.matchers == module.realtime.shard_count * module.capacity.matcher.replicas
    error_message = "Matcher pods must be shards times the modelled replicas."
  }

  # The fleet must divide the partition space, or the binary's last pod takes
  # the remainder and owns more vehicles than its peers.
  assert {
    condition     = module.capacity.fleet.partitions % module.capacity.fleet.size == 0
    error_message = "A fleet of ${module.capacity.fleet.size} does not divide ${module.capacity.fleet.partitions} partitions."
  }

  # Matchers can grow past their floor; the fleet cannot grow at all without
  # re-slicing ownership.
  assert {
    condition     = module.realtime.expected_pod_count.matchers_max > module.realtime.expected_pod_count.matchers
    error_message = "The matcher HPA needs a ceiling above its floor."
  }
}

# A shard is one subject token now. The old scheme split the geohash so a
# wildcard could address a whole cell, which nothing needs any more: the
# orchestrator resolves a shard per event and addresses it directly, and raw and
# matched events are keyed by vehicle partition rather than by geography.
run "a_shard_is_a_single_subject_token" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = module.realtime.subjects.match["r3gq"] == "events.match.r3gq"
    error_message = "Expected events.match.r3gq, got ${module.realtime.subjects.match["r3gq"]}."
  }

  assert {
    condition     = module.realtime.subjects.raw == "events.raw.p.<partition>"
    error_message = "Raw events must be partitioned by vehicle, not by geography."
  }
}

# Every primary must reach the workloads, or the fleet the devstack built is
# larger than the client can address.
run "the_whole_valkey_fleet_reaches_the_workloads" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = length(split(",", module.realtime.redis_value)) == module.capacity.valkey.primaries
    error_message = "REDIS carries ${length(split(",", module.realtime.redis_value))} URLs for ${module.capacity.valkey.primaries} primaries."
  }
}

# The file store is sized from the model, and the matched stream dominates it —
# so a number that looks large is the retention policy, not a mistake.
run "the_jetstream_file_store_is_sized_from_the_model" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = module.capacity.nats.file_store_gib >= 1
    error_message = "The file store must be sized for the retained backlog."
  }

  assert {
    condition     = module.capacity.streams.raw == var.streams
    error_message = "The stream count the orchestrator creates must match the one that was sized."
  }
}

# The known shortfall, pinned so it is visible rather than forgotten.
run "the_outstanding_prerequisite_is_the_matched_stream" {
  command = plan

  expect_failures = [
    check.capacity_is_deliverable,
    check.the_matched_stream_can_absorb_the_emissions,
  ]

  assert {
    condition     = length(keys(output.prerequisites)) == 1
    error_message = "Expected exactly one outstanding prerequisite, got: ${join(", ", keys(output.prerequisites))}."
  }

  assert {
    condition     = contains(keys(output.prerequisites), "matched_stream")
    error_message = "The matched stream shortfall must be reported: ${join(", ", keys(output.prerequisites))}."
  }
}
