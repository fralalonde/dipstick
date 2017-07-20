dipstick
--------
A performant, configurable metrics toolkit for Rust applications.

Provides ergonomic Timer, Counter, Gauges and Event instruments, with optional thread-local scoping. 

Defined metrics are decoupled from implementation so that recorded values can be sent 
transparently to multiple destinations. Current output modules include *Logging* and *Statsd*.
   
Random sampling or local aggregation can be used to reduce the amount of metrics emited by the app.

Publication of aggregated metrics can be done synchronously (i.e. when a program exits) 
or in the background using your favorite scheduler.  

```rust
let metrics_log = LogChannel::new("metrics");
let metrics = DirectDispatch::new(metrics_log);
let counter = metrics.new_count("count_a");
counter.value(1);
```

##TODO
- scopes
- sampling
- tags
- tests
- bench
- doc
- samples