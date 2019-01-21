//! Transient metrics are not retained by buckets after flushing.

extern crate dipstick;

use dipstick::*;

use std::io;
use std::time::Duration;
use std::thread::sleep;


fn main() {
    let bucket = AtomicBucket::new();
    AtomicBucket::default_drain(Stream::write_to(io::stdout()));

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
