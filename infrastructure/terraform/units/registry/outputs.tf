output "repository" {
  description = "For the platform unit, which binds its node account to it."
  value       = module.registry.repository
}

output "image_registry" {
  description = "For the realtime unit's image.registry."
  value       = module.registry.image_registry
}

output "publisher_service_account_email" {
  description = "Give CI this identity through Workload Identity Federation."
  value       = module.registry.publisher_service_account_email
}
