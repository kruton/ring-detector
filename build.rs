use protobuf_codegen;

fn main() {
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .includes(&["dnstap.pb"])
        .input("dnstap.pb/dnstap.proto")
        .cargo_out_dir("protos")
        .run_from_script();
}
