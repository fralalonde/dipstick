#[cfg(feature = "skeptic")]
extern crate skeptic;

fn main() {
    // generates documentation tests.
    #[cfg(feature = "skeptic")]
    skeptic::generate_doc_tests(&["README.md", "HANDBOOK.md"]);
}
