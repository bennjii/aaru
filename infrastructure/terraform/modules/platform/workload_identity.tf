# Lets the named Kubernetes service account impersonate the shard cache reader,
# which is what removes the service account key from the matcher pods.
#
# The bucket and the reader account are the shard-cache module's, and exist
# without a cluster. This binding names the cluster's identity pool
# (`<project>.svc.id.goog`), so it belongs with the cluster. It is by name, and
# holds whether or not the chart has created that Kubernetes account yet.

resource "google_service_account_iam_member" "shard_cache_workload_identity" {
  service_account_id = var.shard_cache_service_account_id
  role               = "roles/iam.workloadIdentityUser"
  member             = "serviceAccount:${var.project_id}.svc.id.goog[${var.workload_identity_namespace}/${var.workload_identity_service_account}]"
}
