use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let routers = [
        manifest_dir.clone() + "/proto/api/v1/router/v1/definition.proto",
        manifest_dir.clone() + "/proto/api/v1/router/v1/service.proto",
        manifest_dir.clone() + "/proto/api/v1/geo.proto",
    ];

    let includes = [manifest_dir.clone() + "/proto"];
    let mut cfg = prost_build::Config::new();
    cfg.bytes(["."]);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    if let Err(e) = tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("routers_descriptor.bin"))
        .protoc_arg("--experimental_allow_proto3_optional")
        .include_file("_includes.rs")
        .compile_protos(&routers, &includes)
    {
        eprintln!("Failed to build. {}", e);
        tonic_build::configure()
            .file_descriptor_set_path(out_dir.join("routers_descriptor.bin"))
            .include_file("_includes.rs")
            .compile_protos(&routers, &includes)?
    }

    Ok(())
}
