use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let routers = [
        "proto/router/v1/definition.proto",
        "proto/router/v1/service.proto",
        "proto/geo.proto",
    ];

    let includes = ["proto"];
    let mut cfg = prost_build::Config::new();
    cfg.bytes(["."]);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    if let Err(e) = tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("routers_descriptor.bin"))
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&routers, &includes)
    {
        eprintln!("Failed to build. {}", e);
        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("routers_descriptor.bin"))
            .compile_protos(&routers, &includes)?
    }

    Ok(())
}
