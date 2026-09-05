terraform {
  required_version = ">= 1.9"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.50"
    }
  }
}

# The backend is generated beside this file by Terragrunt (live/root.hcl), so
# the unit itself has none: `tofu test` and `tofu validate` run on it directly
# with `-backend=false`.
provider "google" {
  project = var.project_id
  region  = var.region
}
