# Guards on the substrate itself, driven directly rather than through an env
# root. Mock providers stand in for GCP, so this runs without credentials.

mock_provider "google" {
  # Mocks invent random strings for computed attributes, and the IAM members
  # validate their `member` with a regexp. Shape the node account like the real
  # thing so the test fails on wiring rather than on the mock.
  mock_resource "google_service_account" {
    defaults = {
      name  = "projects/routers-test/serviceAccounts/dev-gke-nodes@routers-test.iam.gserviceaccount.com"
      email = "dev-gke-nodes@routers-test.iam.gserviceaccount.com"
    }
  }
}

variables {
  project_id                     = "routers-test"
  region                         = "australia-southeast1"
  env                            = "dev"
  cluster_name                   = "routers"
  network_name                   = "routers"
  image_repository               = { location = "australia-southeast1", name = "routers" }
  shard_cache_service_account_id = "projects/routers-test/serviceAccounts/dev-shard-cache@routers-test.iam.gserviceaccount.com"

  subnet_cidr             = "10.0.0.0/20"
  pods_secondary_cidr     = "10.16.0.0/16"
  services_secondary_cidr = "10.32.0.0/20"

  node_pools = {
    matcher = { machine_type = "c4-highcpu-32", min_node_count = 4, max_node_count = 8 }
  }
}

# GKE gives every node a fixed slice of the pod range, sized to twice
# max_pods_per_node rounded up to a power of two. At 110 pods that is a /24, so
# a /16 holds 256 nodes and no more.
run "pod_range_arithmetic_follows_the_slice_size" {
  command = plan

  assert {
    condition     = output.addressing.node_pod_slice_prefix == 24
    error_message = "Expected a /24 slice per node at 110 pods, got /${output.addressing.node_pod_slice_prefix}."
  }

  assert {
    condition     = output.addressing.pod_range_node_capacity == 256
    error_message = "Expected a /16 to hold 256 nodes, got ${output.addressing.pod_range_node_capacity}."
  }
}

# One extra bit of pod range doubles the fleet, which is why widening it early
# is cheap and getting it wrong is not.
run "one_more_bit_doubles_the_node_capacity" {
  command = plan

  variables {
    pods_secondary_cidr = "10.16.0.0/15"
  }

  assert {
    condition     = output.addressing.pod_range_node_capacity == 512
    error_message = "Expected a /15 to hold 512 nodes, got ${output.addressing.pod_range_node_capacity}."
  }
}

# The pod range is fixed for the life of the cluster: GKE cannot re-slice it
# once nodes hold slices. The failure otherwise appears at 3am, when the
# autoscaler stops adding nodes because it cannot allocate a pod range.
run "rejects_a_pod_range_the_pools_can_outgrow" {
  command = plan

  variables {
    node_pools = {
      matcher = { machine_type = "c4-highcpu-32", min_node_count = 100, max_node_count = 300 }
    }
  }

  expect_failures = [google_compute_subnetwork.nodes]
}

# Raising the pod ceiling enlarges every node's slice, so it lowers the node
# count the same range supports. Easy to get backwards.
run "more_pods_per_node_means_fewer_nodes" {
  command = plan

  variables {
    max_pods_per_node = 256
  }

  assert {
    condition     = output.addressing.pod_range_node_capacity == 128
    error_message = "Expected 128 nodes at 256 pods each in a /16, got ${output.addressing.pod_range_node_capacity}."
  }
}

# The GKE API spells taint effects in screaming snake case; a pod spec spells
# them differently. The module translates so the chart can paste the output in.
run "taints_are_translated_for_pod_specs" {
  command = plan

  variables {
    node_pools = {
      matcher = {
        machine_type   = "c4-highcpu-32"
        min_node_count = 1
        max_node_count = 2
        taints = [{
          key    = "routers.dev/pool"
          value  = "matcher"
          effect = "NO_SCHEDULE"
        }]
      }
    }
  }

  assert {
    condition     = output.pool_tolerations["matcher"][0].effect == "NoSchedule"
    error_message = "Expected NoSchedule, got ${output.pool_tolerations["matcher"][0].effect}."
  }

  assert {
    condition     = output.pool_tolerations["matcher"][0].operator == "Equal"
    error_message = "A toleration for a valued taint must use Equal."
  }
}

run "rejects_the_kubernetes_spelling_of_a_taint_effect" {
  command = plan

  variables {
    node_pools = {
      matcher = {
        machine_type   = "c4-highcpu-32"
        min_node_count = 1
        max_node_count = 2
        taints         = [{ key = "k", value = "v", effect = "NoSchedule" }]
      }
    }
  }

  expect_failures = [var.node_pools]
}

# C4 supports Hyperdisk only. A pd-* boot disk there is not the slower choice,
# it is an unavailable one, and it fails during apply — after the cluster
# exists and while its pools are building.
run "rejects_persistent_disk_on_a_hyperdisk_only_series" {
  command = plan

  variables {
    node_disk_type = "pd-balanced"

    node_pools = {
      matcher = {
        machine_type   = "c4-highcpu-32"
        min_node_count = 1
        max_node_count = 2
      }
    }
  }

  expect_failures = [google_container_node_pool.pool["matcher"]]
}

# The same pool is fine once the disk matches what the series offers, so the
# guard is about compatibility rather than about C4 being unusable.
run "accepts_hyperdisk_on_a_hyperdisk_only_series" {
  command = plan

  variables {
    node_disk_type = "hyperdisk-balanced"

    node_pools = {
      matcher = {
        machine_type   = "c4-highcpu-32"
        min_node_count = 1
        max_node_count = 2
      }
    }
  }
}

# Preemption on an untainted pool is silent: pods land there because nothing
# stopped them, and a matcher losing its node takes its shard offline.
run "rejects_an_untainted_spot_pool" {
  command = plan

  variables {
    node_pools = {
      matcher = {
        machine_type   = "c4-highcpu-32"
        min_node_count = 1
        max_node_count = 2
        spot           = true
      }
    }
  }

  expect_failures = [google_container_node_pool.pool["matcher"]]
}

# Autoscaler bounds are cluster-wide totals. The unprefixed pair is per zone,
# which on a three-zone regional cluster would treble every number the capacity
# module produced.
run "autoscaling_bounds_are_totals_not_per_zone" {
  command = plan

  assert {
    condition     = google_container_node_pool.pool["matcher"].autoscaling[0].total_min_node_count == 4
    error_message = "Expected a total floor of 4 nodes."
  }

  assert {
    condition     = google_container_node_pool.pool["matcher"].autoscaling[0].max_node_count == null
    error_message = "The per-zone bound must stay unset, or it fights the total."
  }
}

run "rejects_a_pods_range_smaller_than_a_slash_16" {
  command = plan

  variables {
    pods_secondary_cidr = "10.16.0.0/20"
  }

  expect_failures = [var.pods_secondary_cidr]
}
