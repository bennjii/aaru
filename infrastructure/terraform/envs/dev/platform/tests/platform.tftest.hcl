# Wiring test for the platform root. Mock providers stand in for GCP, so this
# runs with no credentials and no billing: it checks that the modules compose
# and that the guards fire, not that Google accepts the resources.

mock_provider "google" {
  # Mocks invent random strings for computed attributes, and several of these
  # feed arguments the provider validates with a regexp. Shape them like the
  # real thing so the test fails on wiring rather than on the mock.
  # The account id must be 6-30 characters for the IAM member's regexp to
  # accept the resulting resource name.
  mock_resource "google_service_account" {
    defaults = {
      name  = "projects/routers-test/serviceAccounts/dev-shard-cache@routers-test.iam.gserviceaccount.com"
      email = "dev-shard-cache@routers-test.iam.gserviceaccount.com"
    }
  }

  mock_resource "google_artifact_registry_repository" {
    defaults = {
      name = "routers"
    }
  }
}

variables {
  project_id        = "routers-test"
  shard_bucket_name = "routers-test-shards"
}

run "sizing_flows_into_node_pools" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  # A pool per capacity shape, and nothing hand-written.
  assert {
    condition     = length(keys(module.capacity.pools)) == 4
    error_message = "Expected 4 pools, got ${join(", ", keys(module.capacity.pools))}."
  }

  assert {
    condition     = length(module.capacity.unschedulable_shapes) == 0
    error_message = "Every pod shape must fit one node: ${join(", ", module.capacity.unschedulable_shapes)}."
  }

  # The pipeline pool now holds one shape. The historian it used to share with
  # the orchestrators was absorbed into them.
  assert {
    condition     = length(module.capacity.pools["pipeline"].packing) == 1
    error_message = "The pipeline pool should carry only orchestrators, got ${length(module.capacity.pools["pipeline"].packing)} shapes."
  }

  # Matchers autoscale, so their pool's ceiling must exceed its floor; the
  # fleet does not, so its ceiling is only rollout and headroom slack.
  assert {
    condition     = module.capacity.pools["matcher"].max_node_count > module.capacity.pools["matcher"].min_node_count
    error_message = "The matcher pool must have room for its HPA to scale into."
  }
}

# The two quantities a migration cannot avoid. These are the reason the root
# carries a design target at all.
run "the_pinned_wire_law_survives_the_design_target" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = module.capacity.design_target.streams_ok
    error_message = "The pinned ${module.capacity.streams.raw} streams must absorb the design target, at ${module.capacity.design_target.writes_per_stream} writes/s each."
  }

  assert {
    condition     = module.capacity.design_target.fleet_fits
    error_message = "The design target needs a fleet of ${module.capacity.design_target.fleet_required}, past the ${module.capacity.fleet.partitions} partitions available."
  }

  assert {
    condition     = module.capacity.streams.raw_sufficient
    error_message = module.capacity.raw_stream_prerequisite
  }
}

run "the_deployed_precision_agrees_with_the_binaries" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = module.capacity.precision_prerequisite == ""
    error_message = module.capacity.precision_prerequisite
  }

  assert {
    condition     = module.capacity.shard_precision == 4
    error_message = "Expected precision 4, got ${module.capacity.shard_precision}."
  }
}

# The shipped defaults reach every ceiling they can. The one they cannot is the
# matched stream, which is a singleton in `ingest.rs` — so this pins that as
# the *only* outstanding shortfall, and will fail once it is fixed, which is
# when the assertion should become `meets_target`.
run "the_matched_stream_is_the_only_outstanding_shortfall" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = !module.capacity.streams.matched_sufficient
    error_message = "The matched stream now absorbs the target. Split MATCHED_STREAM landed: raise matched_streams and assert meets_target here instead."
  }

  assert {
    condition     = module.capacity.matcher.capacity_eps >= module.capacity.required_eps
    error_message = "Matchers must cover the target:\n${module.capacity.summary}"
  }

  assert {
    condition     = module.capacity.fleet.capacity_eps >= module.capacity.required_eps
    error_message = "The fleet must cover the target:\n${module.capacity.summary}"
  }

  assert {
    condition     = module.capacity.valkey_client_prerequisite == ""
    error_message = module.capacity.valkey_client_prerequisite
  }
}

# The addressing plan has to hold the autoscaler ceiling, not the steady state:
# the range is exhausted at the peak.
run "the_pod_range_holds_the_node_ceiling" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = module.platform.addressing.pod_range_node_capacity >= module.platform.addressing.max_nodes
    error_message = "The pod range holds ${module.platform.addressing.pod_range_node_capacity} nodes but the pools can reach ${module.platform.addressing.max_nodes}."
  }
}
