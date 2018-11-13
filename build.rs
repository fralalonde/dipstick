#[cfg(feature="skeptic")]
extern crate skeptic;

fn main() {
    // generates handbook tests for `README.md`.
    #[cfg(feature="skeptic")]
    skeptic::generate_doc_tests(&["README.md", "HANDBOOK.md"]);

}


