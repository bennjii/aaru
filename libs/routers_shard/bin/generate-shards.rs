use clap::{Args as ClapArgs, Parser};
use geo::Point;
use log::{debug, error, info};
use std::path::PathBuf;

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

    /// The output directory to write shard files to. Defaults to the
    /// workspace's `target/shard_cache` (what the chart mounts).
    #[arg(short, long, env = "SHARD_OUTPUT_DIR")]
    output: Option<PathBuf>,

    /// The name of the manifest file to write.
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

    let partition = ShardedNetwork::<OsmEntryId, OsmEdgeMetadata, _>::partition(
        &source,
        &strategy,
        args.padding,
    );
    let total = partition.len();
    info!("dealt into {total} shards, writing to {out_dir:?}");

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

    let manifest = out_dir.join(args.manifest_filename);
    std::fs::write(&manifest, built.join("\n")).expect("write manifest");

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
