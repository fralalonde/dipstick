#[cfg(feature="skeptic")]
extern crate skeptic;

#[cfg(feature="proto")]
extern crate protoc_rust;

#[cfg(feature="proto")]
use protoc_rust as protoc;

fn main() {
    // generates doc tests for `README.md`.
    #[cfg(feature="skeptic")]
    skeptic::generate_doc_tests(&["README.md"]);

    #[cfg(feature="proto, prometheus")]
    protoc::run(protoc::Args {
        out_dir: "src",
        // "prometheus_proto.rs" is excluded from git
        // FIXME generate to target/gen_src instead
        input: &["schema/prometheus_proto.proto"],
        includes: &[".", "schema"],
        customize: protoc::Customize {
            ..Default::default()
        },
    }).expect("protoc");

}


