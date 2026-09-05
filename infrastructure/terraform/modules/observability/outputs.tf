output "otlp_url" {
  description = "OTLP http/protobuf endpoint. One collector deployment serves the whole pipeline. Derived from the fixed release name, so it is known at plan time."
  value       = "http://otel-collector-opentelemetry-collector.${var.namespace}.svc.cluster.local:4318"
}

output "collector_replicas" {
  value = var.collector_replicas
}

output "release_names" {
  value = concat(
    [helm_release.collector.name],
    [for r in helm_release.prometheus : r.name],
  )
}
