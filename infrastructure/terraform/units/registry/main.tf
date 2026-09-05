# Artifact Registry and its publisher. Cluster-free: apply this on its own to
# work on images and CI without a cluster running. The platform unit reads the
# repository from here to grant its nodes pull access, and the realtime unit
# reads the registry prefix.

module "registry" {
  source = "../../modules/registry"

  project_id     = var.project_id
  region         = var.region
  env            = var.env
  repository_id  = var.repository_id
  immutable_tags = var.immutable_tags
  labels         = var.labels
}
