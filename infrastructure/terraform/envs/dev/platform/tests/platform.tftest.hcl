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

  assert {
    condition     = module.capacity.meets_target
    error_message = "The shipped dev defaults should meet their own target:\n${module.capacity.summary}"
  }

  # Six precision-4 shards over two cells.
  assert {
    condition     = length(module.capacity.cells) == 2
    error_message = "Expected 2 cells from the shipped shard list, got ${length(module.capacity.cells)}."
  }

  # A pool per capacity shape, and nothing hand-written.
  assert {
    condition     = length(keys(module.capacity.pools)) == 4
    error_message = "Expected 4 pools, got ${join(", ", keys(module.capacity.pools))}."
  }

  # A small deployment sits on the availability floor, not on throughput.
  assert {
    condition     = module.capacity.nats.replicas_total == 3
    error_message = "Expected the 3-server NATS floor, got ${module.capacity.nats.replicas_total}."
  }
}

# The pod range is fixed for the life of the cluster, so the precondition that
# checks it has to hold before anything is created.
run "pod_range_holds_the_autoscaler_ceiling" {
  command = plan

  assert {
    condition     = module.platform.addressing.pod_range_node_capacity >= module.platform.addressing.max_nodes
    error_message = "The pod range holds ${module.platform.addressing.pod_range_node_capacity} nodes but the pools scale to ${module.platform.addressing.max_nodes}."
  }
}

# Every pool is tainted, so its nodes stay for their own workload. The module
# converts the GKE spelling to the Kubernetes one for the chart to consume.
run "pool_taints_are_translated_for_pod_specs" {
  command = plan

  assert {
    condition = alltrue([
      for name, tolerations in module.platform.pool_tolerations :
      length(tolerations) == 1 && tolerations[0].effect == "NoSchedule"
    ])
    error_message = "Every pool should expose one NoSchedule toleration: ${jsonencode(module.platform.pool_tolerations)}."
  }

  assert {
    condition = alltrue([
      for name, selector in module.platform.pool_node_selectors :
      selector["routers.dev/pool"] == name
    ])
    error_message = "Node selectors must name their own pool."
  }
}

# The capacity check is what stops a root applying a fleet that cannot carry
# its target. Six precision-4 shards sustain 36k evt/s, so 5M is far out of
# reach and the check must say so rather than let the apply proceed.
run "refuses_a_target_the_shards_cannot_carry" {
  command = plan

  variables {
    throughput_target_eps = 5000000
  }

  expect_failures = [check.capacity_is_deliverable]
}
