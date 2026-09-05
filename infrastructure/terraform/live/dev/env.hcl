# The dev environment. Everything that differs between environments and is not
# a sizing input lives here; the sizing is in sizing.hcl beside it. A new
# environment is a copy of this directory with these values changed.

locals {
  env    = "dev"
  region = "australia-southeast1"

  # The GCP project this environment deploys into. Not a secret, so it is
  # committed rather than read from the shell: one environment, one project.
  # The placeholder is not a valid project id, so a plan fails until it is set.
  project_id = "REPLACE_WITH_GCP_PROJECT_ID"

  cluster_name = "routers"

  # Bucket names are global, hence the project id prefix.
  state_bucket        = "${local.project_id}-routers-tofu-state"
  shard_bucket        = "${local.project_id}-routers-shards"
  shard_public_bucket = "${local.project_id}-routers-shards-public"

  image_repository = "routers"

  # The public CDN copy of the shard cache, for browsers running the
  # WebAssembly component. Off costs nothing; on costs a global forwarding
  # rule (about USD 18/month) plus egress. Set `hostname` once its A record
  # points at the address the shard-cache unit outputs, and HTTPS follows.
  shard_cdn = {
    enabled      = true
    hostname     = ""
    cors_origins = ["*"]
  }

  # Two namespaces, so a teardown of the dependencies cannot touch the routers
  # release and vice versa.
  namespaces = {
    dependencies = "routers-dev"
    realtime     = "routers"
  }

  # Kubernetes service account the realtime workloads run as. The platform unit
  # binds Workload Identity to it by name, so it is declared once, here.
  workload_service_account = "routers-matcher"

  # Mirror a pinned Valkey image into Artifact Registry and set the tag. The
  # Bitnami chart defaults to a floating `latest` from a relocated catalogue,
  # and the valkey module warns on every plan until this is pinned.
  valkey_image = {
    registry   = "${local.region}-docker.pkg.dev"
    repository = "${local.project_id}/routers/valkey"
    tag        = ""
  }

  labels = {
    env    = "dev"
    system = "routers"
  }
}
