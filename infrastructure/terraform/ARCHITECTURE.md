# Scaling architecture

Target: 5M+ raw events per second.

## One hierarchy, two things that ignore it

```
subject   events.position.<cell>.<rest>
                          └─ab─┘ └cdefg┘

shard   full geohash at `shard_precision`   -> one matcher + one orchestrator
cell    first `cell_precision` characters   -> one historian subject, one Helm release

nats    one cluster, sized by delivery rate  -> not partitioned by cell
valkey  one fleet, hashed by vehicle         -> not geographic at all
```

Geohash prefixes nest, so a cell contains shards by construction. Encoding that
into the *subject* rather than only into config is what makes it usable: a NATS
wildcard matches exactly one token, so the two-phase subject gives
`events.position.ab.*` for a whole cell and `events.position.ab.cdefg` for one
shard. With the geohash in a single token neither is expressible, and the
historian would need one subscription — and one pod — per shard.

## Why NATS is one cluster

An earlier revision made each cell a NATS cluster and charged a three-server
availability floor per cell. At precision 3 that was 160 clusters and **480
servers carrying identical traffic**, which made `cell_precision` an expensive
dial and forced the historian to work around it.

That was unnecessary. A core NATS server forwards a message only to routes that
have registered matching subscription interest, and never more than one hop:

> Notably, messages only get routed to servers in the cluster with client
> interest, so are not unnecessarily propogated across a network.
>
> — [nats-general/ARCHITECTURE.md](https://github.com/nats-io/nats-general/blob/main/architecture/ARCHITECTURE.md)

So clustering already partitions by subject. Splitting the cluster bought
locality that interest routing was giving away for free.

One cluster of **25 servers** now carries the same load, and — the part that
matters more than the number — NATS size is independent of `cell_precision`.
Cells can be re-cut without touching the broker, and a producer no longer needs
a cell-to-cluster map.

Two smaller corrections fell out of it:

- **No odd-size rounding.** That was a JetStream meta-group rule applied where
  it does not belong. Core NATS elects nothing, so there is no tiebreak to win.
- **The floor is 3, not 3 per cell**, and it is for availability rather than
  quorum.

### Why not JetStream partitioned streams

JetStream is the obvious next thought, and its documented scaling path is real:
`partition(<buckets>, <tokens...>)` subject mapping splits a subject space into
N streams, so N RAFT leaders spread across the cluster instead of one leader
becoming the bottleneck ([subject
mapping](https://docs.nats.io/nats-concepts/subject_mapping)).

It is still the wrong tool here, because it costs throughput rather than saving
it. Comparing the official `nats bench` figures on identical hardware:

| Mode | Msg size | Throughput |
| --- | --- | --- |
| Core NATS, 1 pub → 4 subs | 128 B | 1,012,200 msgs/s published |
| JetStream async publish, R1 file | 128 B | 403,828 msgs/s |
| JetStream sync publish, R1 memory | 16 B | 35,734 msgs/s |

That is 2.5x slower at best and far worse when a publisher waits for each ack.
No official R3 figure exists at all — NATS publishes no clustered benchmarks —
so a replicated stream would be slower still by an unmeasured margin.

What JetStream would buy is at-least-once delivery and replay. The pipeline
already holds that elsewhere: history lives in Valkey, and that is what a
restarted matcher resumes from.

### The number to distrust

`nats_msgs_per_server` is the least trustworthy input in the model. Every
official NATS throughput figure is a single server over loopback on a laptop,
and the project states outright that it does not benchmark clustering. The
default of 1M deliveries/s is the published 4M aggregate with a 4x haircut,
justified by the one real-hardware datapoint available (roughly 3x under the
published figure for the equivalent benchmark). **Measure it with `nats bench`
on the real node shape before trusting the server count it produces.**

## Why Valkey is not partitioned geographically

The keyspace is `vehicle:<id>:positions`, keyed by **vehicle**. A vehicle
crosses shards while keeping one history stream, and that shared history is
exactly what lets a trip continue across a shard boundary. Partitioning it
geographically would reintroduce the discontinuity it exists to prevent.

So Valkey is one fleet, and a vehicle is placed by **rendezvous hash** over the
primaries' URLs. Two consequences worth knowing:

- **Order does not matter.** A primary's identity is its URL, not its index, so
  `valkey_urls` can be reordered freely.
- **Resizing moves ~1/N of vehicles**, which is the minimum any placement can
  achieve. Plain modulo would have moved nearly all of them, discarding the
  history the design exists to preserve.

An earlier revision assumed ~120k ops/s per primary, from the repo's "valkey is
single-threaded" comments. That is true of command *execution* but not of the
whole server: Valkey 8 moved I/O parsing and writing onto threads.

| Source | Hardware | io-threads | Result |
| --- | --- | --- | --- |
| [1B RPS cluster](https://valkey.io/blog/1-billion-rps/) | 2000x r7g.2xlarge (8 vCPU) | 6 | ~500k ops/s per primary, linear in primary count |
| [Valkey 8 single node](https://valkey.io/blog/unlock-one-million-rps/) | c7g.16xlarge | 8 | 1.19M ops/s, 3.3x Valkey 7.2 |

At 500k ops/s and 2 commands per event, 5M evt/s needs **20 primaries** rather
than 84. `valkey_ops_per_primary` uses the conservative cluster figure, not the
single-node headline: that one has no replication and no cluster bus behind it.

## The historian

`position.<cell>.*` lets one historian cover a whole cell, which drops the
count from one pod per shard (1120) to one per cell (20). But at 20 cells each
would have to absorb ~336k evt/s, over the ~150k one pod sustains.

A queue group resolves it: replicas share the subject's deliveries instead of
duplicating them, so historian count follows load while cells stay coarse.

| Config | historians | Helm releases |
| --- | --- | --- |
| per-cell, 20 cells, no queue group | 20, each saturated | 20 |
| per-cell, 160 cells, no queue group | 160 | 160 |
| per-cell, 20 cells, queue group | 60 | 20 |

Note what this table no longer says. Before NATS became one cluster, the middle
row also cost 480 brokers, which made the queue group close to mandatory. Now
it is merely the better of two workable options.

The model reports `saturated_historian_cells` rather than silently
under-provisioning, and both the chart and the realtime module refuse replicas
above one unless the queue group is set.

## Fleet at 5M evt/s

Standard profile, 1120 shards at precision 6, 20 cells at precision 2,
queue-grouped historians:

| | count |
| --- | --- |
| matcher | 1120 (one per shard, structural) |
| orchestrator | 1120 (one per shard, structural) |
| historian | 60 (3 per cell) |
| NATS | 25 servers, one cluster |
| Valkey | 20 primaries + 20 replicas |
| nodes | 75 matcher, 84 pipeline, 39 infra, 2 system |

## Scaling levers

| Lever | Mechanism | Bounded by |
| --- | --- | --- |
| horizontal, pipeline | more shards, via `shard_precision` | shard file size; trip fragmentation across boundaries |
| vertical, pipeline | more workers per shard, via `vertical_profile` | node vCPU |
| horizontal, valkey | more primaries | resharding moves ~1/N of vehicles |
| vertical, valkey | bigger nodes, more `io-threads` | ~1M ops/s per node |
| horizontal, nats | more servers in the one cluster | full mesh, N(N-1)/2 routes |

Shard count is the pipeline's only horizontal lever, because matcher and
orchestrator subscriptions are plain ephemeral `subscribe()` calls with no queue
group: a second replica on a subject repeats the work.

## Known limits

- **Load is not uniform across shards.** Shards are geographic, traffic is not.
  A single `shard_eps` constant is optimistic, and the busiest shard saturates
  first. Mixed precision is the fix, and needs per-cell `shard_precision`.
- **Matcher and orchestrator have no HA.** One replica per shard means a restart
  takes that shard offline, and matcher startup includes loading the shard file.
  `queue_subscribe` on those two subjects would turn replicas from duplicated
  work into both load-sharing and real availability. This is the largest
  remaining risk.
- **Telemetry needs sampling.** Each event emits several spans, and the
  `BatchSpanProcessor` drops overflow silently from a 16k queue, so spanmetrics
  under-report by an unknown factor. See `telemetry.sampleRatio`.
- **Object count.** ~2300 Deployments makes rollouts an API-server and etcd
  concern, which argues for fatter shards over more of them.
- **Pod IPs.** GKE gives each node a fixed pod slice, so a /16 caps the fleet at
  256 nodes at 110 pods each. The dev root uses a /14. The platform module fails
  the plan when the pools can outgrow the range.
- **The NATS throughput figure is unmeasured.** See above.
