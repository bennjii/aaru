# Inputs to the GCP substrate for the routers realtime pipeline. Sizing is not
# chosen here: `node_pools` comes from the capacity module's `pools` output,
# and this module only turns that shape into infrastructure.

variable "project_id" {
  description = "GCP project that owns the network, cluster, registry and bucket."
  type        = string
}

variable "region" {
  description = <<-EOT
    Region for every regional resource. Keep the bucket and the cluster in the
    same region: a matcher reads its shard cache through the GCS FUSE mount, so
    a cross-region bucket adds latency and egress cost to every cache miss.
  EOT
  type        = string
}

variable "env" {
  description = "Short environment name used in resource names. Short because a service account id is limited to 30 characters and this prefixes several of them."
  type        = string

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]{0,11}$", var.env))
    error_message = "env must be 1-12 characters, lowercase letters, digits and dashes, starting with a letter."
  }
}

variable "cluster_name" {
  description = "Name of the GKE cluster."
  type        = string
}

variable "network_name" {
  description = "Name of the VPC. Subnets are created explicitly, so no default subnet appears."
  type        = string
}

variable "subnet_cidr" {
  description = "Primary range of the node subnet. Only node addresses come from here, so it stays small: pods and services use the secondary ranges below."
  type        = string

  validation {
    condition     = can(cidrhost(var.subnet_cidr, 0))
    error_message = "subnet_cidr must be a valid IPv4 CIDR block."
  }
}

variable "pods_secondary_cidr" {
  description = <<-EOT
    Secondary range for pod IPs. It must be much larger than it first appears
    necessary.

    GKE gives every node a fixed slice of this range, sized to twice
    `max_pods_per_node` rounded up to a power of two, so a node at 110 pods
    consumes a /24 and never releases part of it. Usable node count is the
    number of slices that fit, not the address count: at 110 pods per node a
    /16 holds exactly 256 nodes. The failure appears late — the autoscaler
    stops adding nodes because it cannot allocate a pod range — so size for the
    ceiling. The precondition in network.tf checks this against the sum of
    `max_node_count` over all pools.
  EOT
  type        = string

  validation {
    condition     = can(cidrhost(var.pods_secondary_cidr, 0))
    error_message = "pods_secondary_cidr must be a valid IPv4 CIDR block."
  }

  validation {
    # /16 reaches only 256 nodes at 110 pods per node. Anything smaller cannot
    # host a fleet of this shape, so reject it before a plan is drawn.
    condition     = tonumber(split("/", var.pods_secondary_cidr)[1]) <= 16
    error_message = "pods_secondary_cidr must be /16 or larger; smaller ranges cannot hold enough per-node pod slices."
  }
}

variable "services_secondary_cidr" {
  description = "Secondary range for ClusterIP services. The pipeline uses headless services and direct NATS subjects rather than a service per shard, so this range stays modest even at high shard counts."
  type        = string

  validation {
    condition     = can(cidrhost(var.services_secondary_cidr, 0))
    error_message = "services_secondary_cidr must be a valid IPv4 CIDR block."
  }
}

variable "max_pods_per_node" {
  description = "Pod ceiling per node, applied to every pool and to the cluster default. It also drives the pod range arithmetic: raising it enlarges every node's pod slice and so lowers the node count the range supports."
  type        = number
  default     = 110

  validation {
    condition     = var.max_pods_per_node >= 8 && var.max_pods_per_node <= 256
    error_message = "max_pods_per_node must be between 8 and 256."
  }
}

variable "release_channel" {
  description = "GKE release channel. REGULAR is the usual choice. UNSPECIFIED pins the version by hand and also turns off node auto-upgrade, which then becomes a manual job."
  type        = string
  default     = "REGULAR"

  validation {
    condition     = contains(["RAPID", "REGULAR", "STABLE", "EXTENDED", "UNSPECIFIED"], var.release_channel)
    error_message = "release_channel must be one of RAPID, REGULAR, STABLE, EXTENDED or UNSPECIFIED."
  }
}

variable "master_authorized_cidrs" {
  description = <<-EOT
    Networks allowed to reach the control plane. The endpoint is public but
    closed by default: an empty list means nothing outside Google Cloud reaches
    the API server, including your laptop and CI.
  EOT
  type = list(object({
    cidr_block   = string
    display_name = string
  }))
  default = []
}

variable "master_ipv4_cidr_block" {
  description = "Range for the control plane's own endpoints. It is peered into the VPC and must not overlap the subnet or either secondary range. A /28 is required."
  type        = string
  default     = "172.16.0.0/28"

  validation {
    condition     = can(cidrhost(var.master_ipv4_cidr_block, 0)) && tonumber(split("/", var.master_ipv4_cidr_block)[1]) == 28
    error_message = "master_ipv4_cidr_block must be a valid IPv4 CIDR with a /28 prefix."
  }
}

variable "node_pools" {
  description = <<-EOT
    Node pools to create, keyed by pool name. This shape is the capacity
    module's `pools` output: pass it through rather than hand-writing it.
    `max_node_count` is the ceiling covering headroom plus rollout surge. Set
    `spot` only for work that can lose its node without consequence: a matcher
    owns a shard, so preemption there rebuilds that shard's state.
  EOT
  type = map(object({
    machine_type   = string
    min_node_count = number
    max_node_count = number

    taints          = optional(list(object({ key = string, value = string, effect = string })), [])
    local_ssd_count = optional(number, 0)
    spot            = optional(bool, false)
  }))

  validation {
    condition     = length(var.node_pools) > 0
    error_message = "At least one node pool is required."
  }

  validation {
    condition = alltrue([
      for name, pool in var.node_pools : pool.min_node_count >= 1 && pool.max_node_count >= pool.min_node_count
    ])
    error_message = "Every pool needs min_node_count >= 1 and max_node_count >= min_node_count."
  }

  validation {
    # The GKE API spells taint effects in screaming snake case, unlike the
    # Kubernetes manifests that tolerate them. The module converts to the
    # Kubernetes spelling in the `pool_tolerations` output.
    condition = alltrue(flatten([
      for name, pool in var.node_pools : [
        for t in pool.taints : contains(["NO_SCHEDULE", "PREFER_NO_SCHEDULE", "NO_EXECUTE"], t.effect)
      ]
    ]))
    error_message = "Taint effect must be NO_SCHEDULE, PREFER_NO_SCHEDULE or NO_EXECUTE."
  }

  validation {
    condition = alltrue([
      for name, pool in var.node_pools : pool.local_ssd_count >= 0 && pool.local_ssd_count <= 8
    ])
    error_message = "local_ssd_count must be between 0 and 8."
  }
}

variable "node_disk_size_gb" {
  description = "Boot disk per node. Holds the image layers, and image streaming keeps the working set small."
  type        = number
  default     = 100

  validation {
    condition     = var.node_disk_size_gb >= 50
    error_message = "node_disk_size_gb must be at least 50; smaller disks throttle image pulls."
  }
}

variable "node_disk_type" {
  description = <<-EOT
    Boot disk type.

    `hyperdisk-balanced`, and on C4 there is no alternative: the series does
    not support Persistent Disk at all, so a pd-* value fails at instance
    creation rather than merely being a slower choice. For a series that
    supports only Hyperdisk, the boot disk must itself be Hyperdisk Balanced.

    Nodes still hold no state worth paying for — this is image layers under
    streaming — so the capacity is small and the performance is left at the
    class default.
  EOT
  type        = string
  default     = "hyperdisk-balanced"

  validation {
    condition = contains([
      "pd-standard", "pd-balanced", "pd-ssd",
      "hyperdisk-balanced", "hyperdisk-balanced-high-availability",
    ], var.node_disk_type)
    error_message = "node_disk_type must be a pd-* or hyperdisk-balanced type."
  }
}

variable "artifact_repository_id" {
  description = "Name of the Docker repository that holds the matcher and orchestrator images."
  type        = string
  default     = "routers"
}

variable "immutable_image_tags" {
  description = "Refuse to move a tag that already exists. Worth leaving on: with one replica per shard, a bad image takes that shard offline and a moved tag leaves nothing to roll back to."
  type        = bool
  default     = true
}

variable "shard_bucket_keep_versions" {
  description = "Noncurrent shard file versions retained before deletion."
  type        = number
  default     = 3

  validation {
    condition     = var.shard_bucket_keep_versions >= 1
    error_message = "shard_bucket_keep_versions must be at least 1."
  }
}

variable "shard_bucket_name" {
  description = "Bucket holding the `<shard>.shard.rt` files. Bucket names are global, so this needs a project or organisation prefix to be unique."
  type        = string
}

variable "workload_identity_namespace" {
  description = "Kubernetes namespace of the Helm release whose service account may read the shard bucket."
  type        = string
  default     = "routers"
}

variable "workload_identity_service_account" {
  description = <<-EOT
    Kubernetes service account that matcher pods run as, bound to the shard
    cache Google service account so only pods using it in
    `workload_identity_namespace` read the bucket. The binding is by name, so
    it exists whether or not the chart has created the account yet.
  EOT
  type        = string
  default     = "routers-matcher"
}

variable "deletion_protection" {
  description = <<-EOT
    Guards the cluster and the shard bucket against destroy. Leave it on
    outside scratch environments: `tofu destroy` on a realtime cluster is not
    recoverable from state. It also controls `force_destroy` on the bucket.
  EOT
  type        = bool
  default     = true
}

variable "labels" {
  description = "GCP resource labels for billing and ownership, applied to every resource that accepts them. Not Kubernetes node labels, which the module derives from the pool name."
  type        = map(string)
  default     = {}
}
