# Wiring test for the shard-cache unit. A mock provider stands in for GCP.

mock_provider "google" {
  # The two accounts share one mock. The IAM members validate `member` with a
  # regexp, so the email has to look real; which account it names does not
  # matter to the wiring.
  mock_resource "google_service_account" {
    defaults = {
      name  = "projects/routers-test/serviceAccounts/dev-shard-cache@routers-test.iam.gserviceaccount.com"
      email = "dev-shard-cache@routers-test.iam.gserviceaccount.com"
    }
  }

  mock_resource "google_compute_global_address" {
    defaults = {
      address = "203.0.113.10"
    }
  }
}

variables {
  project_id  = "routers-test"
  env         = "dev"
  bucket_name = "routers-test-shards"

  cdn = {
    enabled     = true
    bucket_name = "routers-test-shards-public"
  }
}

# The matchers' bucket is private whatever the CDN does.
run "the_private_bucket_stays_private" {
  command = plan

  assert {
    condition     = module.shard_cache.bucket == "routers-test-shards"
    error_message = "Unexpected bucket ${module.shard_cache.bucket}."
  }

  assert {
    condition     = output.cdn.bucket != output.shard_bucket
    error_message = "The CDN must front a second bucket, never the matchers' one."
  }
}

# Without a hostname there is no certificate to provision, so the origin is
# plain HTTP on the address.
run "no_hostname_means_http_on_the_address" {
  command = plan

  assert {
    condition     = output.cdn.base_url == "http://203.0.113.10"
    error_message = "Got ${output.cdn.base_url}."
  }

  assert {
    condition     = output.cdn.address == "203.0.113.10"
    error_message = "The address must be exposed so DNS can point at it."
  }
}

run "a_hostname_switches_the_origin_to_https" {
  command = plan

  variables {
    cdn = {
      enabled     = true
      bucket_name = "routers-test-shards-public"
      hostname    = "shards.example.com"
    }
  }

  assert {
    condition     = output.cdn.base_url == "https://shards.example.com"
    error_message = "Got ${output.cdn.base_url}."
  }
}

# An environment with no browser consumers pays for no load balancer.
run "the_cdn_can_be_off" {
  command = plan

  variables {
    cdn = {
      enabled     = false
      bucket_name = ""
    }
  }

  assert {
    condition     = output.cdn == null
    error_message = "With the CDN off the output must be null, got ${jsonencode(output.cdn)}."
  }

  assert {
    condition     = output.shard_bucket == "routers-test-shards"
    error_message = "The private bucket must exist regardless of the CDN."
  }
}

# Reusing the private bucket for the CDN would either fail on public access
# prevention or, if that were relaxed, expose the matchers' bucket.
run "the_same_bucket_for_both_is_refused" {
  command = plan

  variables {
    cdn = {
      enabled     = true
      bucket_name = "routers-test-shards"
    }
  }

  expect_failures = [check.the_public_bucket_is_not_the_private_one]
}
