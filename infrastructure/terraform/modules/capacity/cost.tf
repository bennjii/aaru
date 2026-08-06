# What the sized fleet costs per month.
#
# Kept in the model rather than in a spreadsheet for the same reason the node
# counts are: it is derived from the same pod shapes, so it cannot describe a
# fleet other than the one that would be built. Change the target and the cost
# moves with it.
#
# Every rate is an input, not a constant — see `prices`, which carries the
# region and the date they were read. Prices go stale; arithmetic does not.
#
# Deliberately not modelled: network egress, Cloud Logging ingestion, the
# shard bucket (a few GiB of objects), and Artifact Registry storage. Each is
# small against a fleet this size, and egress in particular is a traffic
# question rather than a capacity one.

locals {
  hours_per_month = 730

  # Cost is quoted at the autoscaler floor, which is the steady state. The
  # matcher pool's ceiling is reported separately: it is the HPA's, and a
  # burst pays for itself only while it lasts.
  pool_monthly = {
    for pool, nodes in local.node_counts :
    pool => nodes * var.prices.machines[var.machines[pool].machine_type][var.pricing_commitment]
  }

  pool_monthly_max = {
    for pool, nodes in local.node_counts_max :
    pool => nodes * var.prices.machines[var.machines[pool].machine_type][var.pricing_commitment]
  }

  compute_monthly     = sum(values(local.pool_monthly))
  compute_monthly_max = sum(values(local.pool_monthly_max))

  # --- Storage ------------------------------------------------------------

  # Every node carries a boot disk, and on a Hyperdisk-only series that disk
  # is Hyperdisk Balanced at the class baseline — no provisioned performance,
  # so capacity only.
  boot_disk_monthly = local.total_nodes * var.boot_disk_gib * var.prices.hyperdisk_balanced_gib

  # The file store, one volume per NATS server. Baseline IOPS and throughput
  # come with the volume, so only the excess is billable — which is most of
  # it here, because the brokers need far more than a baseline volume gives.
  jetstream_capacity_monthly = (
    local.nats_replicas_total * local.file_store_gib_server * var.prices.hyperdisk_balanced_gib
  )

  jetstream_iops_billable = max(0, var.provisioned_iops - var.prices.hyperdisk_free_iops)
  jetstream_mib_billable  = max(0, var.provisioned_throughput_mib - var.prices.hyperdisk_free_mib)

  jetstream_performance_monthly = local.nats_replicas_total * (
    local.jetstream_iops_billable * var.prices.hyperdisk_iops
    + local.jetstream_mib_billable * var.prices.hyperdisk_mib
  )

  jetstream_monthly = local.jetstream_capacity_monthly + local.jetstream_performance_monthly
  storage_monthly   = local.boot_disk_monthly + local.jetstream_monthly

  # --- Cluster ------------------------------------------------------------

  cluster_monthly = var.prices.gke_cluster_hour * local.hours_per_month

  total_monthly     = local.compute_monthly + local.storage_monthly + local.cluster_monthly
  total_monthly_max = local.compute_monthly_max + local.storage_monthly + local.cluster_monthly

  # --- Per service --------------------------------------------------------

  # A pool's nodes are bought whole, so a service's share is its share of the
  # pool's CPU demand. Three of the four pools carry a single service and the
  # attribution is exact; only `infra` splits, between the brokers and the
  # keyspace.
  service_pools = (
    var.pool_layout == "dedicated"
    ? {
      matcher      = "matcher"
      orchestrator = "pipeline"
      nats         = "infra"
      valkey       = "infra"
      telemetry    = "system"
    }
    : {
      matcher      = "shared"
      orchestrator = "shared"
      nats         = "shared"
      valkey       = "shared"
      telemetry    = "shared"
    }
  )

  service_cpu_millis = {
    matcher      = local.matcher_pods * local.profile.matcher_cpu_millis
    orchestrator = local.fleet * local.profile.orchestrator_cpu_millis
    nats         = local.nats_replicas_total * var.nats_cpu_millis
    valkey       = local.valkey_pods_total * var.valkey_cpu_millis
    telemetry = (
      local.collector_replicas * var.collector_cpu_millis
      + (var.observability_enabled ? 4000 : 0)
    )
  }

  service_monthly = {
    for service, pool in local.service_pools :
    service => (
      local.pool_monthly[pool]
      * local.service_cpu_millis[service]
      / local.demand[pool].cpu_millis
    )
  }

  # The storage a service is responsible for, as opposed to the nodes it runs
  # on. Only the brokers hold a volume of their own.
  service_storage_monthly = {
    matcher      = 0
    orchestrator = 0
    nats         = local.jetstream_monthly
    valkey       = 0
    telemetry    = 0
  }

  cost_per_million_events = (
    local.total_monthly / (var.throughput_target_eps * 3600 * local.hours_per_month / 1000000)
  )
}
