fn main() {
    let proto_root = "../../proto";
    let core_common_proto = "../../proto/core/v1/common.proto";
    let core_browser_proto = "../../proto/core/v1/browser.proto";
    let core_tab_proto = "../../proto/core/v1/tab.proto";
    let engine_proto = "../../proto/engine/v1/engine.proto";
    let web_proto = "../../proto/surfaces/web/v1/web.proto";
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to resolve protoc");

    // Safety: build scripts run in a controlled process before compilation.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    println!("cargo:rerun-if-changed={proto_root}");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                core_common_proto,
                core_browser_proto,
                core_tab_proto,
                web_proto,
                engine_proto,
            ],
            &[proto_root],
        )
        .expect("failed to compile engine proto");
}
