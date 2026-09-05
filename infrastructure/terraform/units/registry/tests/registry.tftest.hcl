# Wiring test for the registry unit. A mock provider stands in for GCP.

mock_provider "google" {
  mock_resource "google_artifact_registry_repository" {
    defaults = {
      name = "routers"
    }
  }

  mock_resource "google_service_account" {
    defaults = {
      name  = "projects/routers-test/serviceAccounts/dev-image-publisher@routers-test.iam.gserviceaccount.com"
      email = "dev-image-publisher@routers-test.iam.gserviceaccount.com"
    }
  }
}

variables {
  project_id = "routers-test"
  env        = "dev"
}

# The prefix is what the chart prepends to `routers-matcher` and
# `routers-orchestrator`, so its shape is a contract with the chart.
run "the_registry_prefix_has_the_artifact_registry_shape" {
  command = plan

  assert {
    condition     = output.image_registry == "australia-southeast1-docker.pkg.dev/routers-test/routers"
    error_message = "Got ${output.image_registry}."
  }

  assert {
    condition     = output.repository.location == "australia-southeast1"
    error_message = "The repository must be regional, in the cluster's region."
  }
}

run "tags_are_immutable_by_default" {
  command = plan

  assert {
    condition     = module.registry.repository.name == "routers"
    error_message = "Unexpected repository name ${module.registry.repository.name}."
  }

  assert {
    condition     = var.immutable_tags
    error_message = "A moved tag leaves nothing to roll back to; immutability must be the default."
  }
}
