//! Transient metrics are not retained by buckets after flushing.

use dipstick::*;

use std::thread::sleep;
use std::time::Duration;

fn main() {
    let bucket = AtomicBucket::new();
    AtomicBucket::default_drain(Stream::write_to_stdout());

    let persistent_marker = bucket.marker("persistent");

    let mut i = 0;

    loop {
        i += 1;
        let transient_marker = bucket.marker(&format!("marker_{}", i));

        transient_marker.mark();
        persistent_marker.mark();

        bucket.flush().unwrap();

        sleep(Duration::from_secs(1));
    }
}
