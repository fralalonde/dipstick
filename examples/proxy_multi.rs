//! Use the proxy to send metrics to multiple outputs

/// Create a pipeline that fans out
/// The key here is to use AtomicBucket to read
/// from the proxy and aggregate and flush metrics
///
/// Proxy
///     -> AtomicBucket
///             -> MultiOutput
///                     -> Prometheus
///                     -> Statsd
///                     -> stdout
use dipstick::*;
use std::time::Duration;

metrics! {
    pub PROXY: Proxy = "my_proxy" => {}
}

fn main() {
    // Placeholder to collect output targets
    // This will prefix all metrics with "my_stats"
    // before flushing them.
    let mut targets = MultiInput::new().named("my_stats");

    // Skip the metrics here... we just use this for the output
    // Follow the same pattern for Statsd, Graphite, etc.
    let prometheus = Prometheus::push_to("http://localhost:9091/metrics/job/dipstick_example")
        .expect("Prometheus Socket");
    targets = targets.add_target(prometheus);

    // Add stdout
    targets = targets.add_target(Stream::write_to_stdout());

    // Create the stats and drain targets
    let bucket = AtomicBucket::new();
    bucket.drain(targets);
    // Crucial, set the flush interval, otherwise risk hammering targets
    bucket.flush_every(Duration::from_secs(3));

    // Now wire up the proxy target with the stats and you're all set
    let proxy = Proxy::default();
    proxy.target(bucket.clone());

    // Example using the macro! Proxy sugar
    PROXY.target(bucket.named("global"));

    loop {
        // Using the default proxy
        proxy.counter("beans").count(100);
        proxy.timer("braincells").interval_us(420);
        // global example
        PROXY.counter("my_proxy_counter").count(123);
        PROXY.timer("my_proxy_timer").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(100));
    }
}
