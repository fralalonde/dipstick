# The dipstick handbook
This handbook's purpose is to get you started instrumenting your apps and give an idea of what's possible.
It is not a full-on cookbook (yet) - some reader experimentation may be required!

# Background
Dipstick is a structured metrics library that allows to combine, select from, and switch between multiple metrics backends.
Counters, timers and gauges declared in the code are not tied to a specific implementation. This has multiple benefits.

- Simplified instrumentation: 
For example, a single Counter instrument can be used to send metrics to the log and to a network aggregator. 
This prevents duplication of instrumentation which in turn prevents errors. 

- Flexible configuration:
Let's say we had an application defining a "fixed" metrics stack on initialization. 
We could upgrade to a configuration file defined metrics stack without altering the instruments in every module.
Or we could make the configuration hot-reloadable, suddenly using different output formats.        

- Easier metrics testing:
For example, using a compile-time feature switch, metrics could be collected directly to hash map at test time 
but be sent over the network and written to a file at runtime, as described by external configuration.


# API Overview
Dipstick's API is split between _input_ and _output_ layers.
The input layer provides named metrics such as counters and timers to be used by the application.
The output layer controls how metric values will be recorded and emitted by the configured backend(s).
Input and output layers are decoupled, making code instrumentation independent of output configuration.
Intermediates can also be added between input and output for specific features or performance characteristics.

Although this handbook covers input before output, implementation of metrics can certainly be performed the other way around.

For more details, consult the [docs](https://docs.rs/dipstick/).


## Metrics Input
A metrics library first job is to help a program collect measurements about its operations.

Dipstick provides a restricted but robust set of _five_ instrument types, taking a stance against 
an application's functional code having to pick what statistics should be tracked for each defined metric.
This helps to enforce contracts with downstream metrics systems and keeps code free of configuration elements.
  
### Counters
Counters a quantity of elements processed, for example, the number of bytes received in a read operation. 
Counters only accepts positive values.

### Markers
Markers counters that can only be incremented by one (i.e. they are _monotonic_ counters). 
Markers are useful to count the processing of individual events, or the occurrence of errors.
The default statistics for markers are not the same as those for counters.
Markers offer a safer API than counters, preventing values other than "1" from being passed.  

### Timers
Timers measure an operation's duration.
Timers can be used in code with the `time!` macro,  wrap around a closure or with explicit calls to `start()` and `stop()`.

```rust
extern crate dipstick;
use dipstick::*;
fn main() {
    let metrics = Stream::to_stdout().metrics();
    let timer =  metrics.timer("my_timer");
    
    // using macro
    time!(timer, {/* timed code here ... */} );
    
    // using closure
    timer.time(|| {/* timed code here ... */} );
    
    // using start/stop
    let handle = timer.start();
    /* timed code here ... */
    timer.stop(handle);

    // directly reporting microseconds
    timer.interval_us(123_456);
}
```

Time intervals are measured in microseconds, and can be scaled down (milliseconds, seconds...) on output.
Internally, timers use nanoseconds precision but their actual accuracy will depend on the platform's OS and hardware.

Note that Dipstick's embedded and always-on nature make its time measurement goals different from those of a full-fledged profiler.
Simplicity, flexibility and low impact on application performance take precedence over accuracy.
Timers should still offer more than reasonable performance for most I/O and high-level CPU operations.   
 
### Levels
Levels are relative, cumulative counters.
Compared to counters:
 - Levels accepts positive and negative values.
 - Level's aggregation statistics will track the minimum and maximum _sum_ of values at any point, 
   rather than min/max observed individual values. 
   
```rust
extern crate dipstick;
use dipstick::*;

fn main() {
    let metrics = Stream::to_stdout().metrics();
    let queue_length = metrics.level("queue_length");    
    queue_length.adjust(-2);    
    queue_length.adjust(4);
}
```   

Levels are halfway between counters and gauges and may be preferred to either in some situations.
 
### Gauges
Gauges are use to record instant observation of a resource's value.
Gauges values can be positive or negative, but are non-cumulative.
As such, a gauge's aggregated statistics are simply the mean, max and min values.
Values can be observed for gauges at any moment, like any other metric.    

```rust
extern crate dipstick;
use dipstick::*;

fn main() {
    let metrics = Stream::to_stdout().metrics();
    let uptime = metrics.gauge("uptime");    
    uptime.value(2);    
}
```

### Observers
The observation of values for any metric can be triggered on schedule or upon publication.

This mechanism can be used for automatic reporting of gauge values:
```rust
extern crate dipstick;
use dipstick::*;
use std::time::{Duration, Instant};

fn main() {
    let metrics = Stream::to_stdout().metrics();
    
    // observe a constant value before each flush     
    let uptime = metrics.gauge("uptime");
    metrics.observe(uptime, |_| 6).on_flush();

    // observe a function-provided value periodically     
    let _handle = metrics
        .observe(metrics.gauge("threads"), thread_count)
        .every(Duration::from_secs(1));       
}

fn thread_count(_now: Instant) -> MetricValue {
    6
}
```

Observations triggered `on_flush` take place _before_  metrics are published, allowing last-moment insertion of metric values.

Scheduling could also be used to setup a "heartbeat" metric:
```rust
extern crate dipstick;
use dipstick::*;
use std::time::{Duration};

fn main() {
    let metrics = Graphite::send_to("localhost:2003")
        .expect("Connected")
        .metrics();
    let heartbeat = metrics.marker("heartbeat");
    // update a metric with a constant value every 5 sec
    let _handle = metrics.observe(heartbeat, |_| 1)
        .every(Duration::from_secs(5));
}
```

Scheduled operations can be cancelled at any time using the returned `CancelHandle`. 
Also, scheduled operations are canceled automatically when the metrics input scope they were attached to is `Drop`ped, 
making them more useful with persistent, statically declared `metrics!()`. 

Observation scheduling is done on a best-effort basis by a simple but efficient internal single-thread scheduler.   
Be mindful of measurement callback performance to prevent slippage of following observation tasks.
The scheduler only runs if scheduling is used. 
Once started, the scheduler thread will run a low-overhead wait loop until the application is terminated.    

### Names
Each metric is given a simple name upon instantiation.
Names are opaque to the application and are used only to identify the metrics upon output.

Names may be prepended with a application-namespace shared across all backends.

```rust
extern crate dipstick;
use dipstick::*;
fn main() {   
    let stdout = Stream::to_stdout();
    
    let metrics = stdout.metrics();
    
    // plainly name "timer"
    let _timer = metrics.timer("timer");
    
    // prepend metrics namespace
    let db_metrics = metrics.named("database");
    
    // qualified name will be "database.counter"
    let _db_counter = db_metrics.counter("counter");
}
```

Names may also be prepended with a namespace by each configured backend.
For example, the metric named `success`, declared under the namespace `request` could appear under different qualified names: 
- logging as `app_module.request.success`
- statsd as `environment.hostname.pid.module.request.success`

Aggregation statistics may also append identifiers to the metric's name, such as `counter_mean` or `marker_rate`.

Names should exclude characters that can interfere with namespaces, separator and output protocols.
A good convention is to stick with lowercase alphanumeric identifiers of less than 12 characters.

Note that highly dynamic elements in metric names are usually better handled using `Labels`.

### Labels

Some backends (such as Prometheus) allow "tagging" the metrics with labels to provide additional context,
such as the URL or HTTP method requested from a web server.
Dipstick offers the thread-local ThreadLabel and global AppLabel context maps to transparently carry 
metadata to the backends configured to use it.

Notes about labels:
- Using labels may incur a significant runtime cost because 
  of the additional implicit parameter that has to be carried around. 
- Labels runtime costs may be even higher if async queuing is used 
  since current context has to be persisted across threads.
- While internally supported, single metric labels are not yet part of the input API. 
  If this is important to you, consider using dynamically defined metrics or open a GitHub issue!


### Static vs dynamic metrics
  
Metric inputs are usually setup statically upon application startup.

```rust
extern crate dipstick;
use dipstick::*;

metrics!("my_app" => {
    COUNTER_A: Counter = "counter_a";
});

fn main() {
    Proxy::default_target(Stream::to_stdout().metrics());
    COUNTER_A.count(11);
}
```

The static metric definition macro is just `lazy_static!` wrapper.

## Dynamic metrics

If necessary, metrics can also be defined "dynamically". 
This is more flexible but has a higher runtime cost, which may be alleviated with the optional caching mechanism.

```rust
extern crate dipstick;
use dipstick::*;
fn main() {
    let user_name = "john_day";
    let app_metrics = Log::to_log().cached(512).metrics();
    app_metrics.gauge(&format!("gauge_for_user_{}", user_name)).value(44);
}
```
    
Alternatively, you may use `Labels` to output context-dependent metrics. 

## Metrics Output
A metrics library's second job is to help a program emit metric values that can be used in further systems.

Dipstick provides an assortment of drivers for network or local metrics output.
Multiple outputs can be used at a time, each with its own configuration. 

### Types
These output type are provided, some are extensible, you may write your own if you need to.

#### Stream
Write values to any Write trait implementer, including files, stderr and stdout.

#### Log
Write values to the log using the log crate.

### Map
Insert metric values in a map.  

#### Statsd
Send metrics to a remote host over UDP using the statsd format. 

#### Graphite
Send metrics to a remote host over TCP using the graphite format. 

#### TODO Prometheus
Send metrics to a Prometheus "PushGateway" using the Prometheus 2.0 text format.

### Attributes
Attributes change the outputs behavior.

#### Prefixes
Outputs can be given Prefixes. 
Prefixes are prepended to the Metrics names emitted by this output.
With network outputs, a typical use of Prefixes is to identify the network host, 
environment and application that metrics originate from.       

#### Formatting
Stream and Log outputs have configurable formatting that enables usage of custom templates.
Other outputs, such as Graphite, have a fixed format because they're intended to be processed by a downstream system.

#### Buffering
Most outputs provide optional buffering, which can be used to optimized throughput at the expense of higher latency.
If enabled, buffering is usually a best-effort affair, to safely limit the amount of memory that is used by the metrics.

#### Sampling
Some outputs such as statsd also have the ability to sample metrics.
If enabled, sampling is done using pcg32, a fast random algorithm with reasonable entropy.

```rust
extern crate dipstick;
use dipstick::*;
fn main() {
    let _app_metrics = Statsd::send_to("localhost:8125").expect("connected")
        .sampled(Sampling::Random(0.01))
        .metrics();
}
```

## Intermediates

### Proxy
Because the input's actual _implementation_ depends on the output configuration,
it is necessary to create an output channel before defining any metrics.
This is often not possible because metrics configuration could be dynamic (e.g. loaded from a file),
which might happen after the static initialization phase in which metrics are defined.
To get around this catch-22, Dipstick provides a Proxy which acts as intermediate output, 
allowing redirection to the effective output after it has been set up.


### Bucket
The `AtomicBucket` can be used to aggregate metric values. 
Bucket aggregation is performed locklessly and is very fast.
The tracked statistics vary across metric types:

|       |Counter|Marker | Level | Gauge | Timer |
|-------|-------|---	|---	|---	|---	|
| count |   x	|   x	|   x	|   	|   x	|
| sum  	|   x	|   	|   	|   	|   x	|
| min  	|   x	|   	|   s	|   x	|   x	|
| max  	|   x	|   	|   s	|   x	|   x	|
| rate	|   	|   x	|   	|   	|   x	|
| mean 	|   x	|   	|   x	|   x	|   x	|

Some notes on statistics:

- The count is the "hit count" - the number of times values were recorded.
  If no values were recorded, no statistics are emitted for this metric.

- Markers have no `sum` as it would always be equal to the `count`.
   
- The mean is derived from the sum divided by the count of values. 
  Because count and sum are read sequentially but not atomically, there is _very small_ chance that the 
  calculation could be off in scenarios of high concurrency. It is assumed that the amount of data collected
  in such situations will make up for the slight error.

- Min and max are for individual values except for level where the sum of values is tracked instead.

- The Rate is derived from the sum of values divided by the duration of the aggregation.

#### Preset bucket statistics
Published statistics can be selected with presets such as `all_stats`, `summary`, `average`.

#### Custom bucket statistics
For more control over published statistics, you can provide your own strategy. 
Consult the `custom_publish` [example](https://github.com/fralalonde/dipstick/blob/master/examples/custom_publish.rs) 
to see how this can be done. 


#### Scheduled publication
Buffered and aggregated (bucket) metrics can be scheduled to be 
[periodically published](https://github.com/fralalonde/dipstick/blob/master/examples/bucket_summary.rs) as a background task.
The schedule runs on a dedicated thread and follows a recurrent `Duration`. 
It can be cancelled at any time using the `CancelHandle` returned by the `flush_every()` method.
    
### Multi
Just like Constructicons, multiple metrics channels can assemble, creating a unified facade 
that transparently dispatches metrics to every constituent. 

This can be done using multiple [inputs](https://github.com/fralalonde/dipstick/blob/master/examples/multi_input.rs) 
or multiple [outputs](https://github.com/fralalonde/dipstick/blob/master/examples/multi_output.rs) 

### Asynchronous Queue

Metrics can be collected asynchronously using a queue.
The async queue uses a Rust channel and a standalone thread.
If the queue ever fills up under heavy load, it reverts to blocking (rather than dropping metrics).
I'm sure [an example](https://github.com/fralalonde/dipstick/blob/master/examples/async_queue.rs) would help.

This is a tradeoff, lowering app latency by taking any metrics I/O off the thread but increasing overall metrics reporting latency.
Using async metrics should not be required if using only aggregated metrics such as an `AtomicBucket`. 
