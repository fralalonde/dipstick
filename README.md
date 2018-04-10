# dipstick
A quick, modular metrics toolkit for Rust applications of all types. Similar to popular logging frameworks,
but with counters, markers, gauges and timers.

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick)
[![crates.io](https://img.shields.io/crates/v/dipstick.svg)](https://crates.io/crates/dipstick)
 
Dipstick's main attraction is the ability to send metrics to multiple customized outputs.
For example, metrics could be written immediately to the log _and_ 
sent over the network after a period of aggregation.

Dipstick promotes structured metrics for clean, safe code and good performance.
 
Dipstick builds on stable Rust with minimal dependencies. 

## Features

  - Send metrics to stdout, log, statsd or graphite (one or many)
  - Synchronous, asynchronous or mixed operation
  - Optional fast random statistical sampling
  - Immediate propagation or local aggregation of metrics (count, sum, average, min/max)
  - Periodic or programmatic publication of aggregated metrics
  - Customizable output statistics and formatting
  - Global or scoped (e.g. per request) metrics
  - Per-application and per-output metric namespaces
   
## Examples

For complete applications see the [examples](https://github.com/fralalonde/dipstick/tree/master/examples).

To use Dipstick in your project, add the following line to your `Cargo.toml`
in the `[dependencies]` section:

```toml
dipstick = "0.4.18"
```

Then add it to your code:

```rust,skt-fail,no_run
let metrics = app_metrics(to_graphite("host.com:2003")?);
let counter = metrics.counter("my_counter");
counter.count(3);
```

Send metrics to multiple outputs:

```rust,skt-fail,no_run
let _app_metrics = app_metrics((
        to_stdout(), 
        to_statsd("localhost:8125")?.with_namespace(&["my", "app"])
    ));
```
Since instruments are decoupled from the backend, outputs can be swapped easily.

Aggregate metrics and schedule to be periodical publication in the background:
```rust,skt-run
use std::time::Duration;

let app_metrics = app_metrics(aggregate(all_stats, to_stdout()));
app_metrics.flush_every(Duration::from_secs(3));
```

Aggregation is performed locklessly and is very fast.
Count, sum, min, max and average are tracked where they make sense.
Published statistics can be selected with presets such as `all_stats` (see previous example),
`summary`, `average`.

For more control over published statistics, provide your own strategy:
```rust,skt-run
app_metrics(aggregate(
    |_kind, name, score|
        match score {
            ScoreType::Count(count) => 
                Some((Kind::Counter, vec![name, ".per_thousand"], count / 1000)),
            _ => None
        },
    to_log()));
```

Apply statistical sampling to metrics:
```rust,skt-fail
let _app_metrics = app_metrics(to_statsd("server:8125")?.with_sampling_rate(0.01));
```
A fast random algorithm is used to pick samples.
Outputs can use sample rate to expand or format published data.

Metrics can be recorded asynchronously:
```rust,skt-run
let _app_metrics = app_metrics(to_stdout()).with_async_queue(64);
```
The async queue uses a Rust channel and a standalone thread.
The current behavior is to block when full.

For better performance and easy maintenance, metrics should usually be predefined:
```rust,skt-plain
#[macro_use] extern crate dipstick;
#[macro_use] extern crate lazy_static;
use dipstick::*;

app_metrics!(String, APP_METRICS = app_metrics(to_stdout()));
app_counter!(String, APP_METRICS, {
    COUNTER_A: "counter_a",
});

fn main() {
    COUNTER_A.count(11);
}
```
Metric definition macros are just `lazy_static!` wrappers.


Where necessary, metrics can be defined _ad-hoc_:
```rust,skt-run
let user_name = "john_day";
let app_metrics = app_metrics(to_log().with_cache(512));
app_metrics.gauge(format!("gauge_for_user_{}", user_name)).value(44);
```
Defining a cache is optional but will speed up re-definition of common ad-hoc metrics.

Timers can be used multiple ways:
```rust,skt-run
let app_metrics = app_metrics(to_stdout());
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
let app_metrics = app_metrics(to_stdout());
let db_metrics = app_metrics.with_prefix("database");
let _db_timer = db_metrics.timer("db_timer");
let _db_counter = db_metrics.counter("db_counter");
```

## License

Dipstick is licensed under the terms of the Apache 2.0 and MIT license.

