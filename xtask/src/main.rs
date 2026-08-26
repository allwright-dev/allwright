use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("generate-rust-proto") => generate_rust_proto()?,
        Some(command) => {
            return Err(format!("unknown xtask command: {command}").into());
        }
        None => {
            return Err("usage: cargo run -p xtask -- generate-rust-proto".into());
        }
    }

    Ok(())
}

fn generate_rust_proto() -> Result<(), Box<dyn Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask must live directly under the repo root")?
        .to_path_buf();
    let proto_root = repo_root.join("proto");
    let output_file = repo_root.join("rust/allwright/src/proto_generated.rs");
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    unsafe {
        env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .include_file("proto_generated.rs")
        .out_dir(
            output_file
                .parent()
                .ok_or("proto output file must have a parent directory")?,
        )
        .compile_protos(&[proto_root.join("engine/v1/engine.proto")], &[proto_root])?;

    println!("generated {}", output_file.display());
    Ok(())
}
