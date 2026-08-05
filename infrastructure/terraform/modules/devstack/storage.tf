# The StorageClass behind the JetStream file store.
#
# This exists because the cluster default does not serve the workload. GKE
# defaults to pd-balanced, whose throughput and IOPS scale with capacity — so
# the disk's speed becomes a side effect of how much retention happens to be
# configured, which is exactly the wrong coupling for a broker whose write rate
# is set by traffic.
#
# Hyperdisk provisions both independently of size, so the numbers below come
# from the capacity model's measured demand rather than from the volume being
# large enough to accidentally be fast.
#
# What drives that demand is worth stating: the raw work queues are small
# messages that delete on ack, while the matched stream retains whole cut trips
# for its whole retention window. The matched stream is most of the bytes.

resource "kubernetes_storage_class" "jetstream" {
  count = var.jetstream_storage_class == "" ? 1 : 0

  metadata {
    name = var.jetstream_storage_class_name
  }

  storage_provisioner = "pd.csi.storage.gke.io"

  # The file store is the ingest buffer, not a cache: a work queue holds the
  # only copy of an event until it is acked. Retaining it through a pod's
  # rescheduling is the whole point, so the volume outlives the pod.
  reclaim_policy         = "Retain"
  allow_volume_expansion = true

  # The volume is provisioned in the zone its consumer landed in. With
  # immediate binding a PVC can be created in a zone the NATS pod cannot be
  # scheduled to, and the pod stays Pending against a disk it cannot reach.
  volume_binding_mode = "WaitForFirstConsumer"

  parameters = {
    type = var.jetstream_disk_type

    # Provisioned rather than derived from capacity. Both are per volume, and
    # a NATS server holds one.
    "provisioned-iops-on-create"       = tostring(var.jetstream_provisioned_iops)
    "provisioned-throughput-on-create" = "${var.jetstream_provisioned_throughput_mib}Mi"
  }
}

locals {
  # The class the file store PVCs bind to: the one created here, or an existing
  # class named by the caller.
  jetstream_storage_class = (
    var.jetstream_storage_class != ""
    ? var.jetstream_storage_class
    : kubernetes_storage_class.jetstream[0].metadata[0].name
  )
}

# Provisioning below what the brokers will actually write is the failure this
# module exists to prevent, so it is checked rather than left to a dashboard.
check "the_file_store_can_absorb_the_write_rate" {
  assert {
    condition     = var.jetstream_provisioned_throughput_mib >= var.jetstream_required_throughput_mib
    error_message = "The JetStream file store is provisioned for ${var.jetstream_provisioned_throughput_mib} MiB/s per server but the streams will write ${var.jetstream_required_throughput_mib} MiB/s. Writes will queue behind the disk, the stream's raft leader will fall behind its followers, and ingest will throttle at max_ack_pending while looking like a broker problem."
  }

  assert {
    condition     = var.jetstream_provisioned_iops >= var.jetstream_required_iops
    error_message = "The JetStream file store is provisioned for ${var.jetstream_provisioned_iops} IOPS per server but the streams need ${var.jetstream_required_iops}."
  }
}
