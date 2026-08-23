fn main() {
    let proto_root = "proto";
    let engine_proto = "proto/engine/v1/engine.proto";
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to resolve protoc");

    // Safety: build scripts run in a controlled process before compilation.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    println!("cargo:rerun-if-changed={proto_root}");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[engine_proto], &[proto_root])
        .expect("failed to compile engine proto");
}
