# Derived values shared by the network, cluster and node pool files. Nothing
# here talks to GCP; it is the arithmetic that decides whether the addressing
# plan can hold the fleet the capacity module asked for.

locals {
  name_prefix = "${var.env}-${var.cluster_name}"

  subnet_name         = "${var.network_name}-nodes"
  pods_range_name     = "${var.network_name}-pods"
  services_range_name = "${var.network_name}-services"

  # The autoscaler ceiling across every pool. This, not the steady state, is
  # what the pod range has to survive: the range is exhausted at the peak.
  max_nodes = sum([for name, pool in var.node_pools : pool.max_node_count])

  # GKE reserves a fixed pod slice per node. The slice holds twice
  # max_pods_per_node addresses, rounded up to a power of two, which gives the
  # slice's prefix length. At 110 pods: 2 * 110 = 220, rounded up to 256, so
  # 32 - log2(256) = /24 per node.
  node_pod_slice_prefix = 32 - ceil(log(var.max_pods_per_node * 2, 2))

  pods_prefix = tonumber(split("/", var.pods_secondary_cidr)[1])

  # How many of those slices the pod range contains, and so the hard ceiling
  # on node count. A /16 split into /24s yields 2^(24-16) = 256 nodes. A /15
  # yields 512. Note the doubling: one extra bit of pod range doubles the
  # fleet, so widening the range is cheap and getting it wrong is not.
  pod_range_node_capacity = pow(2, local.node_pod_slice_prefix - local.pods_prefix)

  # Kubernetes node labels, keyed by pool. Deliberately only the pool
  # identity: these become nodeSelectors in the realtime chart, and mixing
  # billing labels into a selector makes a pod's placement depend on a
  # billing change.
  pool_node_labels = {
    for name, pool in var.node_pools : name => {
      "routers.dev/pool" = name
    }
  }

  # The GKE API spells effects NO_SCHEDULE; a Kubernetes toleration spells the
  # same thing NoSchedule. Translate once here so consumers of the
  # `pool_tolerations` output can paste it straight into a pod spec.
  taint_effect_to_kubernetes = {
    NO_SCHEDULE        = "NoSchedule"
    PREFER_NO_SCHEDULE = "PreferNoSchedule"
    NO_EXECUTE         = "NoExecute"
  }

  pool_tolerations = {
    for name, pool in var.node_pools : name => [
      for t in pool.taints : {
        key      = t.key
        operator = "Equal"
        value    = t.value
        effect   = local.taint_effect_to_kubernetes[t.effect]
      }
    ]
  }
}
