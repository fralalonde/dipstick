# dipstick
Configurable structured metrics library for Rust applications. 
Like logging frameworks but with counters, markers, gauges and timers.

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick)
[![crates.io](https://img.shields.io/crates/v/dipstick.svg)](https://crates.io/crates/dipstick)
 
## Features

  - Send metrics to stdout, log, statsd or graphite (one or many)
  - Synchronous or asynchronous operation
  - Local aggregation with periodical publication (scheduled or manual) 
  - Customizable statistics and formatting
  - Global or scoped (e.g. per request) metrics
  - Per-application and per-output metric namespaces
  - Random statistical sampling (statsd)
  - Builds on stable Rust with minimal, feature-gated dependencies.
  
# Goals

Dipstick aims to be the one-stop shop for metrics collection in Rust applications. As such, it strives to offer the nicest, 
Rustiest API, good performance and output drivers for most metrics systems. 

# Anti-goals

Dipstick will not calculate ad-hoc statistics, plot graphs or send alerts. 
It will also not fix your motorcycle (check your coils).   
   
## What it looks like

Here's a basic aggregate & auto-publish counter metric:   

```$rust,skt-run
let metrics = Bucket::new();
metrics.set_target(Text::output(io::stdout()));
metrics.flush_every(Duration::from_secs(3));
let counter = metrics.counter("counter_a");
counter.count(8)
```

Bigger apps will probably want to take advantage of the `metrics!` macro:

```$rust

```

For sample applications see the [examples](https://github.com/fralalonde/dipstick/tree/master/examples).

To use Dipstick in your project, add the following line to your `Cargo.toml`
in the `[dependencies]` section:

```toml
dipstick = "0.7.0-alpha.0"
```

Then add it to your code:

```rust,skt-fail,no_run
let metrics = metric_scope(to_graphite("host.com:2003")?);
let counter = metrics.counter("my_counter");
counter.count(3);
```

Send metrics to multiple outputs:

```rust,skt-fail,no_run
let _app_metrics = metric_scope((
        to_stdout(), 
        to_statsd("localhost:8125")?.with_namespace(&["my", "app"])
    ));
```
Since instruments are decoupled from the backend, outputs can be swapped easily.

Aggregate metrics and schedule to be periodical publication in the background:
```rust,skt-run
use std::time::Duration;

let app_metrics = metric_scope(aggregate());
route_aggregate_metrics(to_stdout());
app_metrics.flush_every(Duration::from_secs(3));
```

Aggregation is performed locklessly and is very fast.
Count, sum, min, max and average are tracked where they make sense.
Published statistics can be selected with presets such as `all_stats` (see previous example),
`summary`, `average`.

For more control over published statistics, provide your own strategy:
```rust,skt-run
metrics(aggregate());
set_default_aggregate_fn(|_kind, name, score|
    match score {
        ScoreType::Count(count) => 
            Some((Kind::Counter, vec![name, ".per_thousand"], count / 1000)),
        _ => None
    });
```

Apply statistical sampling to metrics:
```rust,skt-fail
let _app_metrics = metric_scope(to_statsd("server:8125")?.with_sampling_rate(0.01));
```
A fast random algorithm is used to pick samples.
Outputs can use sample rate to expand or format published data.

Metrics can be recorded asynchronously:
```rust,skt-run
let _app_metrics = metric_scope(to_stdout().with_async_queue(64));
```
The async queue uses a Rust channel and a standalone thread.
The current behavior is to block when full.

For speed and easier maintenance, metrics are usually defined statically:
```rust,skt-plain
#[macro_use] extern crate dipstick;
#[macro_use] extern crate lazy_static;
use dipstick::*;

dispatch_metrics!("my_app" => {
    @Counter COUNTER_A: "counter_a";
});

fn main() {
    route_aggregate_metrics(to_stdout());
    COUNTER_A.count(11);
}
```
Metric definition macros are just `lazy_static!` wrappers.


Where necessary, metrics can be defined _ad-hoc_:
```rust,skt-run
let user_name = "john_day";
let app_metrics = metric_scope(to_log()).with_cache(512);
app_metrics.gauge(format!("gauge_for_user_{}", user_name)).value(44);
```
Defining a cache is optional but will speed up re-definition of common ad-hoc metrics.

Timers can be used multiple ways:
```rust,skt-run
let app_metrics = metric_scope(to_stdout());
let timer =  app_metrics.timer("my_timer");
time!(timer, {/* slow code here */} );
timer.time(|| {/* slow code here */} );

let start = timer.start();
/* slow code here */
timer.stop(start);

timer.interval_us(123_456);
```

Related metrics can share a namespace:
```rust,skt-run
let app_metrics = metric_scope(to_stdout());
let db_metrics = app_metrics.add_prefix("database");
let _db_timer = db_metrics.timer("db_timer");
let _db_counter = db_metrics.counter("db_counter");
```

## License

Dipstick is licensed under the terms of the Apache 2.0 and MIT license.

