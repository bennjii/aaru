use osmpbf::{Element, ElementReader};
use aaru::{Shard};

fn main() {
    let arg = std::env::args_os()
        .nth(1)
        .expect("need a *.osm.pbf file as argument");

    let path = std::path::Path::new(&arg);
    let reader = ElementReader::from_path(path).unwrap();

    println!("Counting...");

    let sharder = Shard::from_file(arg.to_str().unwrap());

    match sharder {
        Ok(shard) => {
            println!("Got Shard! Parent has {} nodes.", shard.data.nodes.len());
        },
        Err(e) => {
            println!("Failed to shard. Reason: {:?}", e);
        }
    }

    return

    // let now = time::Instant::now();
    // let mut nodes = Arc::new(Mutex::new(vec![]));
    //
    // reader.for_each(|item| {
    //     match item {
    //         Element::Way(way) => nodes.clone().lock().unwrap().push(way.clone()),
    //         _ => {}
    //     }
    // }).expect("TODO: panic message");
    //
    // println!("Ways: {}", nodes.clone().lock().unwrap().len());
    //
    // let reader = ElementReader::from_path(path).unwrap();

    match reader.par_map_reduce(
        |element| {
            match element {
                Element::Node(_) => (1, 0, 0, 0),
                Element::DenseNode(_) => (0, 1, 0, 0),
                Element::Relation(_) => (0, 0, 1, 0),
                Element::Way(_) => {
                    (0, 0, 0, 1)
                    // println!("Way::{}", way.tags().map(|tag| format!("{}->{}", tag.0, tag.1)).collect::<Vec<String>>().join(","));
                    // u64::from(way.tags().find(|tag| tag.0.starts_with("highway")).is_some())
                },
            }
        },
        || (0, 0, 0, 0),
        |mut a, mut b| {(a.0+b.0, a.1+b.1, a.2+b.2, a.3+b.3) } // (a.0+b.0, a.1+b.1, a.2+b.2, a.3+b.3)
    ) {
        Ok(total) => {
            println!("Nodes: {}", total.0);
            println!("Nodes (dense): {}", total.1);
            println!("Relations: {}", total.2);
            println!("Ways: {}", total.3);
        }
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        }
    }

    // println!("OP took: {:?}", now.elapsed().as_millis());
}