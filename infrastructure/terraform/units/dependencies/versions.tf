terraform {
  required_version = ">= 1.9"

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.50"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 3.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.38"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

# Read back rather than passed in, so the Helm and Kubernetes providers
# configure themselves from the live cluster instead of from another unit's
# state file.
data "google_container_cluster" "cluster" {
  project  = var.project_id
  name     = var.cluster_name
  location = var.region
}

data "google_client_config" "default" {}

locals {
  cluster_host = "https://${data.google_container_cluster.cluster.endpoint}"
  cluster_ca   = base64decode(data.google_container_cluster.cluster.master_auth[0].cluster_ca_certificate)
}

provider "kubernetes" {
  host                   = local.cluster_host
  cluster_ca_certificate = local.cluster_ca
  token                  = data.google_client_config.default.access_token
}

provider "helm" {
  kubernetes = {
    host                   = local.cluster_host
    cluster_ca_certificate = local.cluster_ca
    token                  = data.google_client_config.default.access_token
  }
}
