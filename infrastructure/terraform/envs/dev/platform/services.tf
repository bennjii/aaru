# APIs the two roots call. Managed here rather than in the modules so a
# module never toggles project-wide state, and in this root rather than
# ../workloads because everything there talks to the cluster, not to GCP.
#
# Bootstrap caveat: enabling a service goes through the Service Usage API,
# which cannot enable itself. On a brand-new project it is on by default; if
# the plan fails citing serviceusage or cloudresourcemanager, run once:
#
#   gcloud services enable serviceusage.googleapis.com cloudresourcemanager.googleapis.com
locals {
  required_services = [
    "compute.googleapis.com",          # network, subnets, node machines
    "container.googleapis.com",        # the GKE cluster and pools
    "artifactregistry.googleapis.com", # image registry
    "storage.googleapis.com",          # shard bucket
    "iam.googleapis.com",              # workload service accounts
    "iamcredentials.googleapis.com",   # workload identity token minting
    "sts.googleapis.com",              # workload identity federation
  ]
}

resource "google_project_service" "required" {
  for_each = toset(local.required_services)

  project = var.project_id
  service = each.value

  # Disabling an API tears down its resources project-wide and can strand
  # other users of the project; a destroy of this stack should not do that.
  disable_on_destroy = false
}
