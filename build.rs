use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = [
        "proto/osm/fileformat.proto",
        "proto/osm/osmformat.proto",
        "proto/mvt/mvt.proto",
    ];

    let includes = [
        "proto"
    ];

    if let Err(e) = prost_build::Config::new()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&protos, &includes) {
        eprintln!("Failed to build. {}", e.to_string());
        prost_build::Config::new().compile_protos(&protos, &includes)?
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    if let Err(e) = tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("aaru_descriptor.bin"))
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(&["proto/aaru.proto"], &["proto"]) {
        eprintln!("Failed to build. {}", e.to_string());
        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("aaru_descriptor.bin"))
            .compile(&["proto/aaru.proto"], &["proto"])?
    }

    Ok(())
}
