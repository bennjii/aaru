variable "chart_path" {
  description = "Path to the routers-realtime chart, normally ../../../chart relative to the env root."
  type        = string
}

variable "release_name" {
  description = "Helm release name. One release holds the whole deployment."
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

# --- Topology ---------------------------------------------------------------

variable "shards" {
  description = <<-EOT
    Geohash shards to deploy, at `shard_precision`. Each renders one matcher
    Deployment serving `events.match.<shard>` through a queue group.
  EOT
  type        = list(string)

  validation {
    condition     = length(var.shards) > 0
    error_message = "At least one shard is required."
  }
}

variable "shard_precision" {
  description = "Geohash precision of `shards`, rendered as the matcher's PRECISION. Must equal the precision compiled into the binaries — see the capacity module's `precision_prerequisite`."
  type        = number
}

variable "streams" {
  description = <<-EOT
    Raw JetStream streams the vehicle partition space divides across.

    Wire law: a revision is a stream sequence, so remapping partitions to a
    different stream count resets sequence domains. It is rendered into the
    orchestrator as STREAMS and must match what the streams were created with.
  EOT
  type        = number
}

variable "fleet_size" {
  description = <<-EOT
    Orchestrator StatefulSet replicas, from the capacity module's `fleet.size`.

    Each pod derives a disjoint slice of the partition space from its ordinal,
    so this is the divisor of that space and not merely a replica count.
    Changing it re-slices ownership across the whole fleet.
  EOT
  type        = number

  validation {
    condition     = var.fleet_size >= 1
    error_message = "fleet_size must be at least 1."
  }
}

variable "matcher_replicas" {
  description = "Replicas per matcher Deployment, and the HPA floor. Members of a shard's queue group split its requests, so this divides load rather than duplicating it."
  type        = number
  default     = 1
}

variable "matcher_replicas_max" {
  description = "HPA ceiling per shard. Above `matcher_replicas` this is the headroom a hot shard may take, which is what covers the gap between a modelled mean and a geographic distribution."
  type        = number
  default     = 4
}

variable "matcher_autoscaling" {
  description = "Whether to render the per-shard HorizontalPodAutoscaler. Meaningful only because matchers queue-subscribe: without a queue group extra replicas would repeat work rather than share it."
  type        = bool
  default     = true
}

variable "matcher_target_cpu_utilization" {
  description = "HPA target CPU. Solving is CPU-bound, so utilisation tracks queue depth closely enough to scale on."
  type        = number
  default     = 80
}

variable "max_shards_per_release" {
  description = "Guard on release size. Every shard renders a Deployment and an HPA into one release, whose manifests live in a single 1 MiB Kubernetes Secret."
  type        = number
  default     = 512
}

# --- Dependencies -----------------------------------------------------------

variable "nats_url" {
  description = "Client URL for the NATS cluster. JetStream must be enabled: the orchestrator creates work-queue streams and filtered durable consumers, which needs server 2.10+."
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
  description = "`pooled-hash` renders every URL; `single` renders only the first and caps the deployment at one primary's throughput."
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
    matcher_workers    = number
    matcher_cpu_millis = number
    matcher_memory_mib = number
    matcher_eps        = number

    orchestrator_workers              = number
    orchestrator_round_trip_ms        = number
    orchestrator_cpu_micros_per_event = number
    orchestrator_cpu_millis           = number
    orchestrator_memory_mib           = number
  })
}

variable "orchestrator_env" {
  description = <<-EOT
    Extra orchestrator env, merged over what this module derives (WORKERS,
    STREAMS, and the sampler pair). Keys are the binary's clap-derived env
    names, so a new knob needs no template change to become deployable.

    CONTEXT_WINDOW is also the resume horizon: a vehicle going more than this
    many events without a committed result loses its reconcile overlap and
    restarts. VEHICLE_CACHE is per worker, so the pod-wide bound is this times
    WORKERS — size it above the concurrent vehicles the pod's partitions carry,
    or a live vehicle's eviction costs a Valkey re-warm and a trip restart.
  EOT
  type        = map(string)
  default = {
    CONTEXT_WINDOW = "25"
    VEHICLE_CACHE  = "4096"

    # The raw tail: the failover recovery source, and the only thing an ack
    # waits on besides the emission's publish.
    HISTORY       = "25"
    BATCH_SIZE    = "128"
    BATCH_TIMEOUT = "20ms"

    # The backlog knob. Under saturation the stream buffers and delivery
    # throttles here; nothing is dropped.
    MAX_ACK_PENDING = "2048"
    ACK_WAIT        = "60s"

    SOLVE_TIMEOUT = "5s"
    SOLVE_RETRIES = "3"

    # How long emissions are retained for the reconciler. Also what sizes that
    # stream's disk, since nothing consumes it destructively.
    MATCHED_RETENTION = "15m"
  }
}

variable "matcher_env" {
  description = "Extra matcher env, merged over the worker count from `profile`."
  type        = map(string)
  default     = {}
}

variable "log_level" {
  description = "RUST_LOG for both services. `debug` is a throughput tax at these rates: the export path itself logs several lines per batch."
  type        = string
  default     = "info"
}

variable "telemetry_sample_ratio" {
  description = <<-EOT
    Trace sample ratio, rendered as the standard OTEL_TRACES_SAMPLER env pair
    through each service's `env` map — the chart passes those maps verbatim, so
    this needs no chart support.

    Unsampled telemetry does not survive this pipeline: the BatchSpanProcessor
    drops the excess silently from a 16k queue, so anything derived from it
    under-reports by an unknown factor. Empty leaves the binaries' own default
    in place.
  EOT
  type        = string
  default     = ""
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
  description = "Tag for both images. Use an immutable tag or a digest."
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
  description = "Node labels pinning the orchestrator fleet to the pipeline pool."
  type        = map(string)
  default     = {}
}

variable "pipeline_tolerations" {
  description = "Tolerations for the pipeline pool's taint."
  type        = list(any)
  default     = []
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

variable "grafana_dashboard_enabled" {
  description = "Whether to ship the dashboard ConfigMap."
  type        = bool
  default     = true
}

# --- Release behaviour ------------------------------------------------------

variable "wait_for_rollout" {
  description = <<-EOT
    Whether Helm waits for pods to become Ready.

    False by default: a few hundred shards is a few hundred Deployments,
    waiting serialises the apply behind the slowest matcher's shard-file load,
    and one unschedulable pod fails the whole release.
  EOT
  type        = bool
  default     = false
}

variable "release_timeout" {
  description = "Release timeout in seconds."
  type        = number
  default     = 900
}

variable "max_history" {
  description = "Helm revisions retained. Each is a Secret holding every rendered manifest, so an unbounded history is real cluster storage at this manifest count."
  type        = number
  default     = 5
}
