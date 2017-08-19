dipstick
--------

[![Build Status](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)](https://travis-ci.org/fralalonde/dipstick.svg?branch=master)

A fast and modular metrics library decoupling app instrumentation from reporting backend.
Similar to popular logging frameworks, but with counters and timers.
Can be configured for combined outputs (log + statsd), random sampling, local aggregation of metrics, recurrent background publication, etc.

```rust
// Send any metrics values directly to the "app_metrics" logger
let mut app_metrics = DirectDispatch::new(LogSink::new("app_metrics"));

// define a timer named "timer_b"
let timer = app_metrics.new_timer("timer_b");

// record time spent in compute_value() using closure or macro syntax
let value1 = timer.time(||, compute_value1());
let value2 = time!(timer, compute_value2());
```

## TODO
- generic publisher / sources
- dispatch scopes
- microsecond-precision intervals
- log templates
- more outputs adapters
- configurable aggregation
- queued dispatch
- non-aggregating buffers
- tagged / ad-hoc metrics
- framework glue (rocket, iron, etc.)
- tests & benchmarks
- complete doc
- examples