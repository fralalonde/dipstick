#[cfg(feature="skeptic")]
extern crate skeptic;

#[cfg(feature="protoc-rust")]
extern crate protoc_rust;

#[cfg(feature="protoc-rust")]
use protoc_rust as protoc;

fn main() {
    // generates doc tests for `README.md`.
    #[cfg(feature="skeptic")]
    skeptic::generate_doc_tests(&["README.md"]);

    #[cfg(feature="protobuf")]
    protoc::run(protoc::Args {
        out_dir: "src",
        input: &["schema/prometheus_proto.proto"],
        includes: &[".", "schema"],
        customize: protoc::Customize {
            ..Default::default()
        },
    }).expect("protoc");

    println!("cargo:rustc-env=RUST_GEN_SRC=../target/gen_src")

}


