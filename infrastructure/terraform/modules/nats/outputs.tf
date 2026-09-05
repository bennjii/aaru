# The Service name is fixed with fullnameOverride, so the URL is derived rather
# than read back from the release. That keeps it known at plan time, which
# matters because the realtime unit feeds it into Helm values.

output "url" {
  description = "Client URL for the cluster. Every workload uses it: one cluster carries the core solve traffic and the JetStream ingest alike."
  value       = "nats://nats.${var.namespace}.svc.cluster.local:4222"
}

output "replicas" {
  description = "Servers in the cluster."
  value       = var.replicas
}

output "storage_class" {
  description = "StorageClass the file store PVCs bind to."
  value       = local.jetstream_storage_class
}

output "release_name" {
  value = helm_release.nats.name
}
