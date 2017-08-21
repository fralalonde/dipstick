dipstick
--------

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)

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

Here's a short example of sending timer values to the log showing both closure and macro syntax
```rust
let mut metrics = metrics(log("app_metrics"));
let timer = metrics.timer("timer_b");
let value1 = timer.time(||, compute_value1());
let value2 = time!(timer, compute_value2());
```

More complete example(s?) can be found in the /examples dir.

## TODO
Although already usable, Dipstick is still under heavy development and makes no guarantees 
of any kind at this point. See the following list for any potential caveats :
- generic publisher / sources
- dispatch scopes
- feature flags
- late builders
- microsecond-precision intervals
- heartbeat metric on publish
- log templates
- more outputs adapters
- configurable aggregation
- queued dispatch
- non-aggregating buffers
- tagged / ad-hoc metrics
- framework glue (rocket, iron, etc.)
- tests & benchmarks
- complete doc
