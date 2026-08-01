variable "chart_path" {
  description = "Path to the routers-realtime chart, normally ../../../chart relative to the env root."
  type        = string
}

variable "release_name" {
  description = "Base Helm release name. Each cell release appends its cell key."
  type        = string
  default     = "routers-realtime"
}

variable "namespace" {
  description = "Namespace for the routers workloads. Kept separate from the dependency namespace."
  type        = string
  default     = "routers"
}

variable "create_namespace" {
  description = "Whether Helm creates the namespace."
  type        = bool
  default     = true
}

# --- Sharding ---------------------------------------------------------------

variable "cell_plan" {
  description = <<-EOT
    Cells to deploy, from the capacity module. Each cell is one Helm release:
    its shards' matchers and orchestrators, plus its historian. A release's
    manifests live in one Kubernetes Secret and a Secret holds 1 MiB, so the
    cell also bounds release size.
  EOT
  type = map(object({
    shards = list(string)
  }))
}

variable "shard_precision" {
  description = "Geohash precision of the shards, passed to the matcher's PRECISION."
  type        = number
}

variable "cell_precision" {
  description = "Geohash prefix length defining a cell, and the first geohash token of every subject."
  type        = number
}

variable "subject_prefix" {
  description = <<-EOT
    Root of the two-phase subject scheme, `<prefix>.position.<cell>.<rest>`. A
    NATS wildcard matches exactly one token, so two geohash tokens are what let
    one subscription address a whole cell.
  EOT
  type        = string
  default     = "events"
}

variable "max_shards_per_cell" {
  description = "Guard on release size. Each shard renders two Deployments, so a cell far past this approaches the 1 MiB Helm release Secret."
  type        = number
  default     = 512
}

# --- Dependencies -----------------------------------------------------------

variable "nats_url" {
  description = "Client URL for the single NATS cluster, from the devstack module. Every cell uses it."
  type        = string
}

variable "valkey_urls" {
  description = <<-EOT
    The Valkey fleet, from the devstack module. Rendered as a comma-separated
    REDIS value. Placement is a rendezvous hash over the URLs, so order does
    not matter and a resize moves about 1/N of vehicles.
  EOT
  type        = list(string)

  validation {
    condition     = length(var.valkey_urls) > 0
    error_message = "At least one Valkey URL is required."
  }
}

variable "valkey_client_mode" {
  description = "`pooled-hash` renders every URL; `single` renders only the first, which is all the current code can use."
  type        = string
  default     = "pooled-hash"

  validation {
    condition     = contains(["single", "pooled-hash"], var.valkey_client_mode)
    error_message = "valkey_client_mode must be 'single' or 'pooled-hash'."
  }
}

variable "otlp_url" {
  description = "OTLP http/protobuf endpoint. Empty disables span export, and telemetry degrades to structured logs."
  type        = string
  default     = ""
}

# --- Workload sizing --------------------------------------------------------

variable "profile" {
  description = "Per-service worker counts and resources, from the capacity module's `profile` output, so the sizing model and the deployment cannot drift apart."
  type = object({
    matcher_workers         = number
    matcher_cpu_millis      = number
    matcher_memory_mib      = number
    orchestrator_workers    = number
    orchestrator_cpu_millis = number
    orchestrator_memory_mib = number
    historian_cpu_millis    = number
    historian_memory_mib    = number
    shard_eps               = number
  })
}

variable "historian_mode" {
  description = <<-EOT
    Subject a historian subscribes to. `per-cell` gives one per cell on
    `<prefix>.position.<cell>.*`, `per-shard` one per shard on an exact
    subject, `global` one on `<prefix>.position.>`.
  EOT
  type        = string
  default     = "per-cell"

  validation {
    condition     = contains(["per-cell", "per-shard", "global"], var.historian_mode)
    error_message = "historian_mode must be 'per-cell', 'per-shard' or 'global'."
  }
}

variable "historian_replicas_by_cell" {
  description = "Historian replicas per cell, from the capacity module. Above one needs `historian_queue_group`."
  type        = map(number)
  default     = {}
}

variable "historian_queue_group" {
  description = <<-EOT
    Whether historians share a subject through a NATS queue group, rendered as
    QUEUE_GROUP. Without it every replica writes every event, and the
    duplicates evict real history under MAXLEN.
  EOT
  type        = bool
  default     = false
}

variable "historian_env" {
  description = <<-EOT
    Historian tuning. BATCH_SIZE is small on purpose: Valkey executes commands
    on one thread, so a large pipeline holds its event loop for tens of
    milliseconds and orchestrator context fetches queue behind it.
  EOT
  type        = map(string)
  default = {
    HISTORY       = "25"
    BATCH_SIZE    = "128"
    BATCH_TIMEOUT = "20ms"
  }
}

variable "orchestrator_env" {
  description = <<-EOT
    Extra orchestrator env, merged over the worker count from `profile`.
    CONTEXT_WINDOW is also the resume overlap horizon; FRESH_CAP bounds the
    per-event matcher work.
  EOT
  type        = map(string)
  default = {
    CONTEXT_WINDOW = "25"
    FRESH_CAP      = "8"
    # Per worker, so the fleet-wide bound is this times the profile's worker
    # count. Bounds trip state, which would otherwise grow for the life of the
    # pod because vehicles leave a shard and never return.
    VEHICLE_CACHE = "1024"
  }
}

variable "matcher_env" {
  description = "Extra matcher env, merged over the worker count derived from `profile`."
  type        = map(string)
  default     = {}
}

variable "log_level" {
  description = "RUST_LOG for all three services. `debug` is a throughput tax at these rates: the export path itself logs several lines per batch."
  type        = string
  default     = "info"
}

# --- Shard cache ------------------------------------------------------------

variable "shard_cache_mode" {
  description = <<-EOT
    Where matchers read their `<shard>.shard.rt` files. `gcsfuse` mounts one
    bucket read-only per pod through the GCS FUSE CSI driver, so a new shard
    needs no volume work. `hostPath` suits only a single-node local cluster.
    `pvc` needs a ReadOnlyMany volume that GCE persistent disks cannot provide
    across zones.
  EOT
  type        = string
  default     = "gcsfuse"

  validation {
    condition     = contains(["gcsfuse", "hostPath", "pvc"], var.shard_cache_mode)
    error_message = "shard_cache_mode must be 'gcsfuse', 'hostPath' or 'pvc'."
  }
}

variable "shard_cache_bucket" {
  description = "GCS bucket holding the shard files. Required when shard_cache_mode is 'gcsfuse'."
  type        = string
  default     = ""
}

variable "shard_cache_host_path" {
  description = "Node directory holding the shard files. Required when shard_cache_mode is 'hostPath'."
  type        = string
  default     = ""
}

variable "shard_cache_pvc_name" {
  description = "Claim name. Required when shard_cache_mode is 'pvc'."
  type        = string
  default     = ""
}

variable "shard_cache_mount_options" {
  description = <<-EOT
    GCS FUSE mount options. `implicit-dirs` lets the flat bucket present as a
    directory. The file cache matters: a matcher reads its whole shard file at
    startup, and a restart would otherwise re-read it over the network.
  EOT
  type        = string
  default     = "implicit-dirs,file-cache:max-size-mb:-1,metadata-cache:ttl-secs:-1"
}

# --- Images -----------------------------------------------------------------

variable "image_registry" {
  description = "Registry/repository prefix, e.g. <region>-docker.pkg.dev/<project>/routers. Empty renders bare image names for a local cluster."
  type        = string
  default     = ""
}

variable "image_tag" {
  description = "Tag for all three images. Use an immutable tag or a digest: with one replica per shard there is no second replica to compare against when a bad image lands."
  type        = string
  default     = "latest"
}

variable "image_pull_policy" {
  description = "Pull policy. `Never` suits a local cluster with pre-built images; GKE needs `IfNotPresent` or `Always`."
  type        = string
  default     = "IfNotPresent"
}

variable "image_pull_secrets" {
  description = "Pull secret names. Not needed for Artifact Registry, where the node service account authenticates."
  type        = list(string)
  default     = []
}

# --- Scheduling -------------------------------------------------------------

variable "matcher_node_selector" {
  description = "Node labels pinning matchers to their pool. Matching is CPU-bound, so it wants a compute-optimised shape of its own."
  type        = map(string)
  default     = {}
}

variable "matcher_tolerations" {
  description = "Tolerations for the matcher pool's taint, which keeps other workloads off it."
  type        = list(any)
  default     = []
}

variable "pipeline_node_selector" {
  description = "Node labels pinning orchestrators and historians to the pipeline pool."
  type        = map(string)
  default     = {}
}

variable "pipeline_tolerations" {
  description = "Tolerations for the pipeline pool's taint."
  type        = list(any)
  default     = []
}

variable "priority_class_name" {
  description = "PriorityClass for all three services. A shard's matcher is the only one there is, so an eviction takes that shard offline until it reschedules."
  type        = string
  default     = ""
}

# --- Service account --------------------------------------------------------

variable "service_account_name" {
  description = "Kubernetes service account for the workloads. Required for gcsfuse, which authenticates to GCS through Workload Identity."
  type        = string
  default     = ""
}

variable "service_account_create" {
  description = "Whether the chart creates the service account."
  type        = bool
  default     = true
}

variable "gcp_service_account_email" {
  description = "GCP service account to impersonate through Workload Identity. Rendered as the iam.gke.io/gcp-service-account annotation."
  type        = string
  default     = ""
}

# --- Telemetry --------------------------------------------------------------

variable "telemetry_sample_ratio" {
  description = <<-EOT
    Trace sample ratio, rendered as the standard OTEL_TRACES_SAMPLER env vars.
    Empty samples everything. The chart's values.yaml notes the caveat about
    whether the Rust SDK honours these.
  EOT
  type        = string
  default     = ""
}

variable "grafana_dashboard_enabled" {
  description = "Whether to ship the dashboard ConfigMap. Rendered once, not once per cell."
  type        = bool
  default     = true
}

# --- Release behaviour ------------------------------------------------------

variable "wait_for_rollout" {
  description = <<-EOT
    Whether Helm waits for pods to become Ready. False by default: a cell of
    128 shards is 384 Deployments, waiting serialises the apply behind the
    slowest matcher's shard-file load, and one unschedulable pod fails the
    whole release.
  EOT
  type        = bool
  default     = false
}

variable "release_timeout" {
  description = "Per-release timeout in seconds."
  type        = number
  default     = 900
}

variable "max_history" {
  description = "Helm revisions retained per release. Each is a Secret, so an unbounded history is real cluster storage at this release count."
  type        = number
  default     = 5
}
