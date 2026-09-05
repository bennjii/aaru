# Run through `just terraform::lint`, which lints every module and unit in its
# own directory. Terragrunt's live/ tree holds no .tf files and is not linted
# here; `terragrunt hcl validate` covers it.

config {
  # Each directory is linted on its own, so module calls are not followed.
  call_module_type = "none"
}

plugin "terraform" {
  enabled = true
  preset  = "recommended"
}

# Catches invalid machine types, disk types and regions before a plan reaches
# GCP. `tflint --init` downloads it; the version is pinned so a lint result is
# reproducible.
plugin "google" {
  enabled = true
  version = "0.39.0"
  source  = "github.com/terraform-linters/tflint-ruleset-google"
}
