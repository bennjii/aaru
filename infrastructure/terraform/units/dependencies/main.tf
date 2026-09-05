# In-cluster dependencies: NATS with JetStream, the Valkey fleet, and the
# telemetry path — the otel-collector, plus kube-prometheus-stack when enabled.
#
# Applied after platform, which owns the cluster this points at, and before
# realtime, which takes this unit's URLs through a Terragrunt dependency. It
# holds its own state so a rollout of the routers images replans nothing here,
# and a teardown here cannot touch the routers release.

module "nats" {
  source = "../../modules/nats"

  namespace = var.namespace

  replicas                 = module.capacity.nats.replicas_total
  jetstream_file_store_gib = module.capacity.nats.file_store_gib

  # Capacity and performance are provisioned separately, because they are
  # separate constraints: the volume is sized for the retention window and
  # provisioned for the write rate. A class whose speed scales with its size
  # would tie the two together.
  jetstream_required_throughput_mib    = module.capacity.jetstream_disk.write_mib_per_server
  jetstream_required_iops              = module.capacity.jetstream_disk.iops_per_server
  jetstream_provisioned_throughput_mib = var.jetstream_provisioned_throughput_mib
  jetstream_provisioned_iops           = var.jetstream_provisioned_iops

  jetstream_instance_iops_limit           = var.jetstream_instance_iops_limit
  jetstream_instance_throughput_limit_mib = var.jetstream_instance_throughput_limit_mib

  # The brokers and the keyspace share the infra pool; telemetry has its own.
  # Placement comes from the platform unit's outputs, so a pool's taint and its
  # workloads' tolerations cannot drift apart.
  node_selector = var.pool_node_selectors["infra"]
  tolerations   = var.pool_tolerations["infra"]
}

module "valkey" {
  source = "../../modules/valkey"

  namespace = var.namespace

  primaries  = module.capacity.valkey.primaries
  io_threads = module.capacity.valkey.io_threads
  image      = var.valkey_image

  node_selector = var.pool_node_selectors["infra"]
  tolerations   = var.pool_tolerations["infra"]
}

module "observability" {
  source = "../../modules/observability"

  namespace = var.namespace

  collector_replicas      = module.capacity.telemetry.collector_replicas
  prometheus_enabled      = var.prometheus_enabled
  grafana_anonymous_admin = var.grafana_anonymous_admin

  node_selector = var.pool_node_selectors["system"]
  tolerations   = var.pool_tolerations["system"]
}
