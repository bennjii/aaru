# Infrastructure as code

OpenTofu modules, composed into three apply units, driven per environment by
Terragrunt.

```
modules/        reusable, no providers configured, no state
  capacity/     the sizing model: pure arithmetic, tested with `tofu test`
  registry/     Artifact Registry and the image publisher account
  shard-cache/  the private shard bucket, its reader and publisher accounts
  shard-cdn/    a public copy of the shard cache behind Cloud CDN, for browsers
  platform/     VPC, GKE cluster, node pools, and the bindings onto the above
  nats/         NATS with JetStream and the file store StorageClass
  valkey/       the Valkey fleet, one release per primary
  observability/ otel-collector and kube-prometheus-stack
  realtime/     the routers-realtime Helm release

units/          one apply unit each; environment-agnostic; has providers + tests
  registry/     registry                       (cluster-free)
  shard-cache/  shard-cache + shard-cdn        (cluster-free)
  platform/     capacity + platform            (the cluster)
  dependencies/ capacity + nats + valkey + observability
  realtime/     capacity + realtime            (changes on every rollout)

live/           per environment: values only, no .tf files
  root.hcl      remote state (GCS), optional state encryption, common inputs
  dev/
    env.hcl     project, region, names, namespaces
    sizing.hcl  the capacity model's inputs, declared once for every unit
    shards.txt  one geohash shard per line
    registry/ shard-cache/ platform/ dependencies/ realtime/   a terragrunt.hcl each
```

## Why five units

The rule is: anything that runs without the cluster is its own unit, so it can
be applied, changed and torn down without one. The registry and the shard
cache (with its CDN) cost next to nothing and are what image and shard work
needs, so each stands alone. The platform holds the cluster and the two
bindings that tie it to those units: node pull access on the repository, and
Workload Identity on the shard cache reader. The dependencies change when the
sizing changes. The realtime release changes on every image tag. Each holds its
own state, so a rollout replans one Helm release, and a teardown of one unit
cannot touch another.

Values flow between units through Terragrunt `dependency` blocks. The platform
unit reads the repository and the reader account from the two cluster-free
units. The realtime unit reads the registry prefix from `registry`, the bucket
and reader from `shard-cache`, the pool placement from `platform`, and the
NATS, Valkey and OTLP URLs from `dependencies`. Nothing is copied by hand.

## Why every unit runs the sizing model

`units/*/sizing.tf` is byte-identical across the units, and `just validate`
fails if it is not. Each unit recomputes the model from `live/<env>/sizing.hcl`
rather than reading another unit's outputs, because the model is pure
arithmetic and the inputs are the same object. The pools that were built, the
brokers that were installed and the fleet that was deployed therefore agree by
construction. The realtime unit also checks the Valkey fleet it was handed
against its own model, so a dependencies apply from a stale sizing is caught at
plan.

## Working with it

Everything runs through `just`. From the repository root, prefix with
`terraform::`.

```
just terraform                      # fmt-check, validate, lint, test; no credentials needed
just terraform::bootstrap dev       # once: create the state bucket
just terraform::apply dev registry      # cluster-free; on its own
just terraform::apply dev shard-cache   # cluster-free; on its own
just terraform::plan dev platform       # one unit
just terraform::apply dev platform
just terraform::plan-all dev        # every unit, in dependency order
just terraform::summary dev         # the sizing report
just terraform::cost dev            # Infracost estimate of the platform unit
```

A rollout is `ROUTERS_IMAGE_TAG=<digest> just terraform::apply dev realtime`,
or pin the tag in `live/dev/realtime/terragrunt.hcl` so the repository records
what is deployed.

Before the first apply, set `project_id` in `live/dev/env.hcl`. Nothing else
is required; bucket names derive from it.

The shard CDN is on in dev and serves plain HTTP on a global address until
`shard_cdn.hostname` in `env.hcl` names a DNS record pointed at that address,
after which a managed certificate provisions and the origin is HTTPS. Set
`shard_cdn.enabled = false` in an environment with no browser consumers.
`generate-shards` uploads as the publisher account to both buckets.

## A new environment

Copy `live/dev` to `live/<name>`, change `env.hcl`, and edit `sizing.hcl` for
that environment's target. The units do not change.

## Testing

`modules/capacity/tests` pins the model's numbers. `modules/platform/tests` and
`units/*/tests` use mock providers, so they run with no cloud account and check
wiring, not that Google accepts the resources. Several runs declare
`check.capacity_is_deliverable` as an expected failure: the shipped 800k target
is not fully met by the current binaries (the matched stream is a singleton),
and the tests pin that as the only outstanding shortfall so it is visible
rather than forgotten.

## Cost

The former hand-maintained cost model is replaced by Infracost, which prices
the platform unit's plan with live rates. `just terraform::cost` generates a
usage file from the plan so the node count is the autoscaler floor the model
produced, not the `initial_node_count` the autoscaler immediately overrides.
Not covered: the JetStream file store volumes (created by the CSI driver, not
OpenTofu), per-service attribution, and committed-use discounts.
`cost-baseline` and `cost-diff` compare two sizings.

## State

State lives in a GCS bucket per environment, one prefix per unit, with native
locking. Set `ROUTERS_STATE_KMS_KEY` to a Cloud KMS key to encrypt state and
plan files; see `live/root.hcl`.

Local development on OrbStack does not go through this tree: see
`infrastructure/dev` and `infrastructure/chart`.
