terraform {
  required_version = ">= 1.8"

  required_providers {
    # GA provider only. Every feature this module uses (Dataplane V2, the GCS
    # FUSE CSI driver addon, Workload Identity, the autoscaling profile) is
    # generally available in google 6.x, so there is no google-beta dependency
    # to carry. Keep it that way: a beta provider forces a second provider
    # block on every root module that consumes this one.
    google = {
      source  = "hashicorp/google"
      version = "~> 6.50"
    }
  }
}
