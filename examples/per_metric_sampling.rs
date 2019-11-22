//! A sample application sending ad-hoc marker values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    let statsd = Statsd::send_to("localhost:8125")
        .expect("Connected")
        .named("my_app");
    // Sampling::Full is the default
    // .sampled(Sampling::Full);

    let unsampled_marker = statsd.metrics().marker("marker_a");

    let hi_freq_marker = statsd
        .sampled(Sampling::Random(0.1))
        .metrics()
        .marker("low_freq_marker");

    let lo_freq_marker = statsd
        .sampled(Sampling::Random(0.001))
        .metrics()
        .marker("hi_freq_marker");

    loop {
        unsampled_marker.mark();

        for _i in 0..10 {
            lo_freq_marker.mark();
        }
        for _i in 0..1000 {
            hi_freq_marker.mark();
        }
        std::thread::sleep(Duration::from_millis(3000));
    }
}
