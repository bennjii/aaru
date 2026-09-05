# Wiring test for the dependencies unit. Mock providers stand in for GCP, Helm
# and Kubernetes, so this runs with no cluster: it checks that the sizing model
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
  project_id = "routers-test"

  # The valkey module refuses an unpinned image, because the Bitnami chart
  # default is a floating tag from a relocated catalogue.
  valkey_image = {
    registry   = "australia-southeast1-docker.pkg.dev"
    repository = "routers-test/routers/valkey"
    tag        = "8.1.1-debian-12-r0"
  }

  # What the platform unit outputs for the pools the dev sizing produces.
  pool_node_selectors = {
    infra  = { "routers.dev/pool" = "infra" }
    system = { "routers.dev/pool" = "system" }
  }
  pool_tolerations = {
    infra  = [{ key = "routers.dev/pool", operator = "Equal", value = "infra", effect = "NoSchedule" }]
    system = [{ key = "routers.dev/pool", operator = "Equal", value = "system", effect = "NoSchedule" }]
  }

  sizing = {
    shards                = ["r3gq", "r3gr", "r3gw", "r3gx", "r652", "r658"]
    shard_precision       = 4
    throughput_target_eps = 800000
    design_target_eps     = 5000000
    streams               = 64
    vertical_profile      = "standard"
    machines = {
      matcher  = { machine_type = "c4-highcpu-32", vcpu = 32, memory_gib = 64 }
      pipeline = { machine_type = "c4-highcpu-16", vcpu = 16, memory_gib = 32 }
      infra    = { machine_type = "c4-standard-16", vcpu = 16, memory_gib = 60 }
      system   = { machine_type = "c4-standard-8", vcpu = 8, memory_gib = 30 }
    }
  }
}

# Every run deploys the shipped 800k target, which the current binaries cannot
# fully absorb (the matched stream is a singleton in `ingest.rs`), so the
# capacity check is declared as an expected failure. When that is fixed these
# expectations start failing, which is the prompt to delete them.

run "the_brokers_are_sized_from_the_model" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = module.nats.replicas == module.capacity.nats.replicas_total
    error_message = "The NATS cluster must have the modelled server count: ${module.nats.replicas} against ${module.capacity.nats.replicas_total}."
  }

  # Odd, because a stream's leader is elected.
  assert {
    condition     = module.nats.replicas % 2 == 1
    error_message = "A JetStream cluster wants an odd server count, got ${module.nats.replicas}."
  }

  assert {
    condition     = output.totals.nats_servers == module.nats.replicas
    error_message = "The totals must report the deployed server count."
  }
}

# The file store is sized from the model, and the matched stream dominates it —
# so a number that looks large is the retention policy, not a mistake. The
# provisioned performance has to sit above the modelled write rate.
run "the_file_store_is_provisioned_above_the_modelled_write_rate" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = module.capacity.nats.file_store_gib >= 1
    error_message = "The file store must be sized for the retained backlog."
  }

  assert {
    condition     = var.jetstream_provisioned_throughput_mib >= module.capacity.jetstream_disk.write_mib_per_server
    error_message = "Provisioned ${var.jetstream_provisioned_throughput_mib} MiB/s is below the modelled ${module.capacity.jetstream_disk.write_mib_per_server} MiB/s per server."
  }

  assert {
    condition     = var.jetstream_provisioned_iops >= module.capacity.jetstream_disk.iops_per_server
    error_message = "Provisioned ${var.jetstream_provisioned_iops} IOPS is below the modelled ${module.capacity.jetstream_disk.iops_per_server} per server."
  }
}

# Every primary must be addressable, or the fleet is larger than the client can
# reach. The realtime unit re-checks this count against its own model.
run "the_whole_valkey_fleet_is_addressable" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  assert {
    condition     = length(output.valkey_urls) == module.capacity.valkey.primaries
    error_message = "Got ${length(output.valkey_urls)} URLs for ${module.capacity.valkey.primaries} primaries."
  }

  assert {
    condition     = length(distinct(output.valkey_urls)) == length(output.valkey_urls)
    error_message = "Valkey URLs must be distinct; a duplicate would place two hash slots on one primary."
  }
}

run "every_release_is_accounted_for" {
  command = plan

  expect_failures = [check.capacity_is_deliverable]

  # nats, one release per primary, the collector and kube-prometheus-stack.
  assert {
    condition     = length(output.releases) == 1 + module.capacity.valkey.primaries + 2
    error_message = "Expected ${1 + module.capacity.valkey.primaries + 2} releases, got ${join(", ", output.releases)}."
  }
}
