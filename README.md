dipstick
--------
A configurable, extendable application metrics framework. 
Similar to popular logging frameworks, but with counters and timers. 
Enables multiple outputs (i.e. Log file _and_ Statsd) from a single set of instruments.
Supports random sampling, local aggregation and periodical publication of collected metric values.

Configuration can be simple:

```rust
    let mut app_metrics = DirectDispatch::new(LogSink::new("metrics:"));
    let timer = app_metrics.new_timer("timer_b");
```

Or it can be more involved if you require a more sophisticated setup:

```rust
    // send application metrics to both aggregator and to sampling log
    let aggregator = MetricAggregator::new();
    let sampling_log = RandomSamplingSink::new(LogSink::new("metrics:"), 0.1);
    let dual_sink = DualSink::new(aggregator.sink(), sampling_log);

    // schedule aggregated metrics to be sent to statsd every 3 seconds
    let statsd = MetricCache::new(StatsdSink::new("localhost:8125", "hello.").unwrap(), 512);
    let aggregate_metrics = AggregatePublisher::new(statsd, aggregator.source());
    // TODO publisher should provide its own scheduler
    CoreExecutor::new().unwrap().schedule_fixed_rate(
        Duration::from_secs(3),
        Duration::from_secs(3),
        move |_| aggregate_metrics.publish()
    );
    
    let mut app_metrics = DirectDispatch::new(dual_sink);
```

And here's a  API for app code starts with a MetricDispatch implementation wrapping over a previously defined chain of MetricSinks.

```rust
    // define application metrics    
    let counter = app_metrics.new_count("counter_a");
    let timer = app_metrics.new_timer("timer_b");
    let event = app_metrics.new_event("event_c");
    let gauge = app_metrics.new_gauge("gauge_d");

    loop {
        // report some metric values from our "application" loop
        counter.value(11);
        gauge.value(22);

        // use scope to update metrics as one (single log line, single network packet, etc.)
        app_metrics.scope(|| {
            event.mark();
            time!(timer, { sleep(Duration::from_millis(5)); });
        });
    }
```

##TODO
- generic publisher / sources
- scope properties
- microsecond-precision intervals 
- log templates
- more outputs
- configurable aggregates
- buffers & queued dispatch
- tags
- tests & benches
- create doc
- lots of sample code