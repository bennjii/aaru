# Shared by every unit under live/. Each terragrunt.hcl includes it and gets the
# remote state, the optional state encryption and the environment-wide inputs
# from here instead of declaring them.

locals {
  env = read_terragrunt_config(find_in_parent_folders("env.hcl")).locals
}

# One GCS bucket per environment, one prefix per unit. The first run needs
# `--backend-bootstrap` (see `just terraform::bootstrap`) to create the bucket
# with versioning on; after that the GCS backend locks natively and nothing
# here needs a hand.
remote_state {
  backend = "gcs"

  generate = {
    path      = "backend.tf"
    if_exists = "overwrite_terragrunt"
  }

  config = {
    project  = local.env.project_id
    location = local.env.region
    bucket   = local.env.state_bucket
    prefix   = path_relative_to_include()

    enable_bucket_policy_only = true
  }
}

# State encryption, opt-in. The dependencies and realtime units hold a
# short-lived cluster token in state from `google_client_config`, and the
# platform unit holds the cluster CA. Point ROUTERS_STATE_KMS_KEY at a Cloud
# KMS key (projects/../locations/../keyRings/../cryptoKeys/..) to encrypt state
# and plan files with it. Unset, this generates nothing. Existing unencrypted
# state is still read after enabling, and is written back encrypted.
generate "encryption" {
  path      = "encryption.tf"
  if_exists = "overwrite_terragrunt"
  disable   = get_env("ROUTERS_STATE_KMS_KEY", "") == ""

  contents = <<-EOF
    terraform {
      encryption {
        key_provider "gcp_kms" "state" {
          kms_encryption_key = "${get_env("ROUTERS_STATE_KMS_KEY", "")}"
          key_length         = 32
        }

        method "aes_gcm" "state" {
          keys = key_provider.gcp_kms.state
        }

        state {
          method = method.aes_gcm.state
        }

        plan {
          method = method.aes_gcm.state
        }
      }
    }
  EOF
}

# Every unit takes these. The rest of a unit's inputs live in its own
# terragrunt.hcl; the cluster name is the platform unit's, and the units that
# run on the cluster read it from that unit's outputs.
inputs = {
  project_id = local.env.project_id
  region     = local.env.region
}
