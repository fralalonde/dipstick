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

Dipstick provides a restricted but robust set of _four_ instrument types, taking a stance against 
an application's functional code having to pick what statistics should be tracked for each defined metric.
This helps to enforce contracts with downstream metrics systems and keeps code free of configuration elements.
  
#### Counter
Count number of elements processed, e.g. number of bytes received. Only accepts positive amounts.

#### Marker 
A monotonic counter. e.g. to record the processing of individual events.
Default aggregated statistics for markers are not the same as those for counters.
Value-less metric also makes for a safer API, preventing values other than 1 from being passed.  

#### Timer
Measure an operation's duration.
Usable either through the time! macro, the closure form or explicit calls to start() and stop().
While timers internal precision are in nanoseconds, their accuracy depends on platform OS and hardware. 
Timer's internal precision is microseconds but can be scaled down on output.
 
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

### Level
A relative, cumulative quantity counter. Accepts positive and negative values.
If aggregated, observed minimum and maximum track the _sum_ of values (as opposed to `Counter` min and max _individual_ values). 
 
### Gauge
An instant observation of a resource's value (positive or negative, but non-cumulative).
The observation of Gauges can be performed programmatically as with other metric types or 
it can be triggered automatically, either on schedule or upon flushing the scope:

````rust
extern crate dipstick;
use dipstick::*;
use std::time::Duration;

fn main() {
    let metrics = Stream::to_stdout().metrics();
    let uptime = metrics.gauge("uptime");
    
    // report gauge value programmatically
    uptime.value(2);
    
    // observe a constant value before each flush     
    let uptime = metrics.gauge("uptime");
    metrics.observe(uptime, || 6).on_flush();

    // observe a function-provided value periodically     
    metrics
        .observe(metrics.gauge("threads"), thread_count)
        .every(Duration::from_secs(1));       
}

fn thread_count() -> MetricValue {
    6
}
````

### Names
Each metric must be given a name upon creation.
Names are opaque to the application and are used only to identify the metrics upon output.

Names may be prepended with a application-namespace shared across all backends.

```rust
extern crate dipstick;
use dipstick::*;
fn main() {   
    let metrics = Stream::to_stdout().metrics();
    
    // plainly name "timer"
    let _timer = metrics.timer("timer");
    
    // prepend namespace
    let db_metrics = metrics.named("database");
    
    // qualified name will be "database.counter"
    let _db_counter = db_metrics.counter("counter");
}
```

Names may be prepended with a namespace by each configured backend.
For example, the same metric `request.success` could appear under different qualified names: 
- logging as `app_module.request.success`
- statsd as `environment.hostname.pid.module.request.success`

Aggregation statistics may also append identifiers to the metric's name, such as `counter_mean` or `marker_rate`.

Names should exclude characters that can interfere with namespaces, separator and output protocols.
A good convention is to stick with lowercase alphanumeric identifiers of less than 12 characters.


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

<<<<<<< HEAD
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
Another intermediate output is the Bucket, which can be used to aggregate metric values. 
Bucket-aggregated values can be used to infer statistics which will be flushed out to

Bucket aggregation is performed locklessly and is very fast.
Count, Sum, Min, Max and Mean are tracked where they make sense, depending on the metric type.

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
