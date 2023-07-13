extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["dnstap.pb/dnstap.proto"], &["protos/"]).unwrap();
}
