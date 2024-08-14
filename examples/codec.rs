use std::fs::File;
use std::io::{BufReader, Read};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use tokio::time::Instant;
use aaru::codec::BlobIterator;
use aaru::codec::consts::{AUSTRALIA};

// struct GreedyReader {
//
// }

#[tokio::main]
async fn main() {
    let now = Instant::now();
    let path = PathBuf::from(AUSTRALIA);
    let mut blob_iter = BlobIterator::new(path).unwrap();

    let f = File::open(path).unwrap();
    let size = f.metadata().unwrap().size() as usize;
    let mut reader = BufReader::new(f);
    let mut buf = Vec::with_capacity(size);
    reader.read_to_end(&mut buf);

    println!("{:?}: {}", &buf[0..=5], buf.len());

    // let elements = blob_iter.into_iter()
    //     .map(|blob| {
    //         blob.item.datasize
    //     })
    //     .reduce(
    //         |a, b| a + b
    //     );
    //
    // assert_eq!(elements, Some(16864038));

    println!("Time taken: {}µs ({}ms)", now.elapsed().as_micros(), now.elapsed().as_micros() / 1000)
}