# dipstick

A configurable structured metrics library for Rust applications. 
Like logging frameworks but with counters, markers, gauges and timers.

[![crates.io](https://img.shields.io/crates/v/dipstick.svg)](https://crates.io/crates/dipstick)
[![docs.rs](https://docs.rs/dipstick/badge.svg)](https://docs.rs/dipstick)
[![downloads](https://img.shields.io/crates/d/dipstick.svg)](https://crates.io/crates/dipstick)

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick)
[![license-mit](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/fralalonde/dipstick/blob/master/LICENSE-MIT)
[![license-apache](http://img.shields.io/badge/license-APACHE-blue.svg)](https://github.com/fralalonde/dipstick/blob/master/LICENSE-APACHE)

[![Join the chat at https://gitter.im/fralalonde/dipstick](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/fralalonde/dipstick?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
[![Coverage Status](https://coveralls.io/repos/fralalonde/dipstick/badge.svg?branch=master)](https://coveralls.io/r/fralalonde/dipstick?branch=master)

![a dipstick picture](https://raw.githubusercontent.com/fralalonde/dipstick/master/assets/dipstick_red_small.jpg)

Dipstick aims to be the one-stop shop for metrics _collection_ and _routing_ in rust applications.
As such, it strives to offer:
 - the nicest, most ergonomic metrics configuration and collection APIs
 - the best performance with less impact on applications
 - the greatest choice of outputs to popular metrics systems


Dipstick builds on stable Rust with minimal, feature-gated dependencies.

## Features

Because dipstick is packaged as a toolkit,
applications configure it to suit their needs. 
Dipstick-enabled apps _can_:

  - Send metrics to console, log, statsd, graphite or prometheus (one or many)
  - Serve metrics over HTTP
  - Locally aggregate metrics values
  - Publish aggregated metrics, on schedule or programmatically 
  - Customize output statistics and formatting
  - Define global or scoped (e.g. per request) metrics
  - Statistically sample metrics (statsd)
  - Choose between sync or async operation
  - Choose between buffered or immediate output
  - Switch between metric backends at runtime 
  
### Non-goals

By design, dipstick will not
- calculate ad-hoc statistics from previously recorded data
- plot graphs
- send alerts
- track histograms
  
These are all best done by downstream software,
such as timeseries visualization and monitoring tools.   

## Show me the code!

Here's a basic aggregating & auto-publish counter metric:
   
```$rust,skt-run
let bucket = Bucket::new();
bucket.set_target(Text::output(io::stdout()));
bucket.flush_every(Duration::from_secs(3));
let counter = bucket.counter("counter_a");
counter.count(8)
```

Persistent apps wanting to declare static metrics will prefer using the `metrics!` macro:

```$rust,skt-run
metrics! { METRICS = "my_app" =>
    pub COUNTER: Counter = "my_counter";
}

fn main() {
    METRICS.set_target(Graphite::output("graphite.com:2003").unwrap());
    COUNTER.count(32);
} 
```

For sample applications see the [examples](https://github.com/fralalonde/dipstick/tree/master/examples).
For documentation see the [handbook](https://github.com/fralalonde/dipstick/tree/master/handbook).

To use Dipstick in your project, add the following line to your `Cargo.toml`
in the `[dependencies]` section:

```toml
dipstick = "0.7.0"
```

## License

Dipstick is licensed under the terms of the Apache 2.0 and MIT license.

