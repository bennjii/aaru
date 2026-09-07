use clap::{Args as ClapArgs, Parser};
use geo::Point;
use log::{debug, error, info};
use std::path::PathBuf;

extern crate alloc;
use alloc::collections::BTreeSet;

use routers_codec::osm::{OsmEdgeMetadata, OsmEntryId, OsmNetwork};
use routers_network::edge::Weight;
use routers_shard::{GeohashStrategy, ShardSource, ShardedNetwork};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path to the PBF/RT file to load.
    #[command(flatten)]
    file: FileInput,

    /// The precision of the geohash strategy to use.
    #[arg(short, long, env, default_value = "4")]
    precision: u8,

    /// Metres of cross-boundary buffer admitted around each shard.
    #[arg(long, env = "PADDING_DISTANCE", default_value = "1000.0")]
    padding: f64,

    /// Only write shards whose id starts with this prefix. For chunked
    /// extracts: a chunk cut with a buffer around a geohash cell yields
    /// shards outside that cell too, which belong to the neighbouring chunk.
    #[arg(long, env = "ONLY_PREFIX")]
    only_prefix: Option<String>,

    /// The output directory to write shard files to. Defaults to the
    /// workspace's `target/shard_cache` (what the chart mounts).
    #[arg(short, long, env = "SHARD_OUTPUT_DIR")]
    output: Option<PathBuf>,

    /// The name of the manifest file to write. An existing manifest is
    /// merged into, not replaced, so several regions can share one output
    /// directory.
    #[arg(short, long, env = "MANIFEST_FILENAME", default_value = "manifest.txt")]
    manifest_filename: String,
}

#[derive(ClapArgs, Debug)]
#[group(required = true, multiple = false)]
struct FileInput {
    /// The path to the PBF file to load.
    #[arg(long)]
    pbf: Option<PathBuf>,

    /// The path to the RT file to load.
    #[arg(long)]
    rt: Option<PathBuf>,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    info!("generate-shards starting: {:?}", args);

    let out_dir = args.output.unwrap_or_else(|| {
        // `cargo run` sets CARGO_MANIFEST_DIR to libs/routers_shard, so the
        // fallback lands in the workspace's target/shard_cache.
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default())
            .join("../../target/shard_cache")
    });
    std::fs::create_dir_all(&out_dir).expect("create output dir");

    let network = match (args.file.pbf, args.file.rt) {
        (Some(pbf), None) => {
            info!("loading OsmNetwork from protobuf file...");
            OsmNetwork::from_pbf(&pbf).map_err(|v| v.to_string())
        }
        (None, Some(rt)) => {
            info!("loading OsmNetwork from cached (.rt) file...");
            OsmNetwork::from_saved(&rt)
        }
        _ => unreachable!(),
    }
    .expect("must be able to parse the provided file");

    debug!(
        "file contained {} nodes, {} edges",
        network.hash.len(),
        network.graph.edge_count()
    );

    let strategy = GeohashStrategy::with_precision(args.precision);
    let source = OsmSource(&network);

    // Indices are not serialised, so building them here would be wasted.
    let mut partition = ShardedNetwork::<OsmEntryId, OsmEdgeMetadata, _>::partition(
        &source,
        &strategy,
        args.padding,
    )
    .without_indices();
    let dealt = partition.len();
    if let Some(prefix) = &args.only_prefix {
        partition.retain(|id| id.to_string().starts_with(prefix));
        info!(
            "dealt into {dealt} shards, keeping the {} under prefix {prefix:?}",
            partition.len()
        );
    }
    let total = partition.len();
    info!("writing {total} shards to {out_dir:?}");

    let mut built = Vec::with_capacity(total);
    let mut failed = Vec::new();
    for (i, net) in partition.enumerate() {
        let name = format!("{}.shard.rt", net.owned);
        debug!("[{} / {total}] {net:?}", i + 1);
        match net.save_to_file(&out_dir.join(&name)) {
            Ok(()) => built.push(name),
            Err(e) => {
                error!("[{} / {total}] failed to save {name}: {e}", i + 1);
                failed.push((name, e));
            }
        }
    }

    // Merge into any existing manifest: the output directory is shared by
    // every region generated into it, and a rerun must not duplicate lines.
    let manifest = out_dir.join(args.manifest_filename);
    let mut names: BTreeSet<String> = match std::fs::read_to_string(&manifest) {
        Ok(existing) => existing
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeSet::new(),
        Err(e) => panic!("read existing manifest {manifest:?}: {e}"),
    };
    let before = names.len();
    names.extend(built.iter().cloned());
    let mut contents = names.iter().cloned().collect::<Vec<_>>().join("\n");
    contents.push('\n');
    std::fs::write(&manifest, contents).expect("write manifest");
    info!(
        "manifest {manifest:?}: {} entries ({} new)",
        names.len(),
        names.len() - before
    );

    info!(
        "{} failed, {} shards built into {out_dir:?}",
        failed.len(),
        built.len()
    );
}

// Thin wrapper around the network to allow iterating over the values
struct OsmSource<'a>(&'a OsmNetwork);

impl<'a> ShardSource<OsmEntryId, OsmEdgeMetadata> for OsmSource<'a> {
    fn nodes<'b>(&'b self) -> Box<dyn Iterator<Item = (OsmEntryId, Point)> + 'b> {
        Box::new(self.0.hash.values().map(|n| (n.id, n.position)))
    }

    fn edges<'b>(
        &'b self,
    ) -> Box<dyn Iterator<Item = (OsmEntryId, OsmEntryId, Weight, OsmEdgeMetadata)> + 'b> {
        Box::new(
            self.0
                .graph
                .all_edges()
                .filter_map(|(from, to, (weight, edge_id))| {
                    let meta = self.0.meta.get(&edge_id.index())?.clone();
                    Some((from, to, *weight, meta))
                }),
        )
    }
}
