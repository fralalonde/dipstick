[![crates.io](https://img.shields.io/crates/v/dipstick.svg)](https://crates.io/crates/dipstick)
[![docs.rs](https://docs.rs/dipstick/badge.svg)](https://docs.rs/dipstick)
[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick)

# dipstick ![a dipstick picture](https://raw.githubusercontent.com/fralalonde/dipstick/master/assets/dipstick_single_ok_horiz_transparent_small.png)
A one-stop shop metrics library for Rust applications with lots of features,  
minimal impact on applications and a choice of output to downstream systems.

## Features
Dipstick is a toolkit to help all sorts of application collect and send out metrics.
As such, it needs a bit of set up to suit one's needs.
Skimming through the [handbook](https://github.com/fralalonde/dipstick/tree/master/HANDBOOK.md)
and many [examples](https://github.com/fralalonde/dipstick/tree/master/examples)
should help you get an idea of the possible configurations.

In short, dipstick-enabled apps _can_:

  - Send metrics to console, log, statsd, graphite or prometheus (one or many)
  - Locally aggregate the count, sum, mean, min, max and rate of metric values
  - Publish aggregated metrics, on schedule or programmatically
  - Customize output statistics and formatting
  - Define global or scoped (e.g. per request) metrics
  - Statistically sample metrics (statsd)
  - Choose between sync or async operation
  - Choose between buffered or immediate output
  - Switch between metric backends at runtime

For convenience, dipstick builds on stable Rust with minimal, feature-gated dependencies.
Performance, safety and ergonomy are also prime concerns.

### Non-goals
Dipstick's focus is on metrics collection (input) and forwarding (output).
Although it will happily aggregate base statistics, for the sake of simplicity and performance Dipstick will not
- plot graphs
- send alerts
- track histograms

These are all best done by downstream timeseries visualization and monitoring tools.

## Show me the code!
Here's a basic aggregating & auto-publish counter metric:

```rust
use dipstick::*;

fn main() {
    let bucket = AtomicBucket::new();
    bucket.drain(Stream::write_to_stdout());
    bucket.flush_every(std::time::Duration::from_secs(3));
    let counter = bucket.counter("counter_a");
    counter.count(8);
}
```

Persistent apps wanting to declare static metrics will prefer using the `metrics!` macro:

```rust
use dipstick::*;

metrics! { METRICS = "my_app" => {
        pub COUNTER: Counter = "my_counter";
    }
}

fn main() {
    METRICS.target(Graphite::send_to("localhost:2003").expect("connected").metrics());
    COUNTER.count(32);
}
```

For sample applications see the [examples](https://github.com/fralalonde/dipstick/tree/master/examples).
For documentation see the [handbook](https://github.com/fralalonde/dipstick/tree/master/HANDBOOK.md).

To use Dipstick in your project, add the following line to your `Cargo.toml`
in the `[dependencies]` section:

```toml
dipstick = "0.9.0"
```

## External features

Configuring dipstick from a text file is possible using 
the [spirit-dipstick](https://crates.io/crates/spirit-dipstick) crate.  

## Building
When building the crate prior to PR or release, just run plain old `make`. 
This will in turn run `cargo` a few times to run tests, benchmarks, lints, etc.
Unfortunately, nightly Rust is still required to run `bench` and `clippy`.    

## TODO / Missing / Weak points
- Prometheus support is still primitive (read untested). Only the push gateway approach is supported for now. 
- No backend for "pull" metrics yet. Should at least provide tiny-http listener capability.  
- No quick integration feature with common frameworks (Actix, etc.) is provided yet.
- Thread Local buckets could be nice.
- "Rolling" aggregators would be nice for pull metrics. Current bucket impl resets after flush.

## License
Dipstick is licensed under the terms of the Apache 2.0 and MIT license.
