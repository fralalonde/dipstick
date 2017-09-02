dipstick
--------

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick)

A fast and modular metrics library decoupling app instrumentation from reporting backend.
Similar to popular logging frameworks, but with counters and timers.
Can be configured for combined outputs (log + statsd), random sampling, local aggregation of metrics, recurrent background publication, etc.

## Design
Dipstick's design goals are to:
- support as many metrics backends as possible while favoring none
- support all types of applications, from embedded to servers
- promote metrics conventions that facilitate app monitoring and maintenance
- stay out of the way in the code and at runtime (ergonomic, fast, resilient)

## Code
Here's an example showing usage of a predefined timer with the closure syntax. 
Each timer value is :
- Written immediately to the "app_metrics" logger.
- Sent to statsd immediately, one time out of ten (sampled). 
```rust
use dipstick::*;

let app_metrics = metrics((
    log("app_metrics"),
    sample(0.1, statsd("stats:8125"))
    ));
    
let timer = app_metrics.timer("timer_b");
loop {
    let value2 = time!(timer, compute_value2());
}
```

In this other example, an ad-hoc timer with macro syntax is used.
- Each new timer value is aggregated with the previous values.
- Aggregation tracks count, sum, max and min values (locklessly).
- Aggregated scores are written to log every 10 seconds.  
- `cache(sink)` is used to prevent metrics of the same to be created multiple times.  
```rust
use dipstick::*;
use std::time::Duration;

let (sink, source) = aggregate();
let app_metrics = metrics(cache(sink));
publish(source, log("last_ten_seconds")).publish_every(Duration::from_secs(10));

loop {
    let value2 = time!(app_metrics.timer("timer_b"), compute_value2());
}
```

Other example(s?) can be found in the /examples dir.

## TODO 
Although already usable, Dipstick is still under heavy development and makes no guarantees 
of any kind at this point. See the following list for any potential caveats :
- META turn TODOs into GitHub issues
- generic publisher / sources
- dispatch scopes
- feature flags
- non-tokio scheduler
- late builders
- microsecond-precision intervals
- heartbeat metric on publish
- logger templates
- more outputs adapters
- configurable aggregation
- queued dispatch
- non-aggregating buffers
- tagged / ad-hoc metrics
- framework glue (rocket, iron, etc.)
- tests & benchmarks
- complete doc / inline samples
- example applications
- make a cool logo 
