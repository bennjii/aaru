fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/helloworld.proto")?;

    let protos = [
        "proto/osm/fileformat.proto",
        "proto/osm/osmformat.proto"
    ];

    let includes = [
        "proto"
    ];

    prost_build::compile_protos(&protos, &includes)?;

    Ok(())
}
