# dipstick

A fast and modular metrics toolkit for all Rust applications. 
Similar to popular logging frameworks, but with counters and timers.

- Does not bind application code to a single metrics implementation.
- Builds on stable Rust with minimal dependencies.

```rust
use dipstick::*;
let app_metrics = metrics(to_stdout());
app_metrics.counter("my_counter").count(3);
```

Metrics can be sent to multiple outputs at the same time.
```rust
let app_metrics = metrics((to_log("log_this:"), to_statsd("localhost:8125")));
```
Since instruments are decoupled from the backend, outputs can be swapped easily.     
 
Metrics can be aggregated and sent periodically in the background.
```rust
use std::time::Duration;
let (to_aggregate, from_aggregate) = aggregate();
publish_every(Duration::from_secs(10), from_aggregate, to_log("last_ten_secs:"), all_stats);
let app_metrics = metrics(to_aggregate);
```
Use predefined publishing strategies `all_stats`, `summary`, `average` or roll your own. 

Metrics can be statistically sampled.
```rust
let app_metrics = metrics(sample(0.001, to_statsd("localhost:8125")));
```

Metrics can be recorded asynchronously.
```rust
let app_metrics = metrics(async(to_stdout()));
```

Metric definitions can be cached to make using ad-hoc metrics faster.
```rust
let app_metrics = metrics(cache(512, to_log()));
app_metrics.gauge(format!("my_gauge_{}", 34)).value(44);
```

Timers can be used multiple ways.
```rust
let timer =  app_metrics.timer("my_timer");
time!(timer, {/* slow code here */} );
timer.time(|| {/* slow code here */} );

let start = timer.start();
/* slow code here */
timer.stop(start);

timer.interval_us(123_456);
```

Related metrics can share a namespace.
```rust
let db_metrics = app_metrics.with_prefix("database.");
let db_timer = db_metrics.timer("db_timer");
let db_counter = db_metrics.counter("db_counter"); 
```

## Design
Dipstick's design goals are to:
- support as many metrics backends as possible while favoring none
- support all types of applications, from embedded to servers
- promote metrics conventions that facilitate app monitoring and maintenance
- stay out of the way in the code and at runtime (ergonomic, fast, resilient)

## Performance
Predefined timers use a bit more code but are generally faster because their initialization cost is is only paid once.
Ad-hoc timers are redefined "inline" on each use. They are more flexible, but have more overhead because their init cost is paid on each use. 
Defining a metric `cache()` reduces that cost for recurring metrics.    

Run benchmarks with `cargo +nightly bench --features bench`.

## TODO 
Although already usable, Dipstick is still under heavy development and makes no guarantees 
of any kind at this point. See the following list for any potential caveats :
- META turn TODOs into GitHub issues
- generic publisher / sources
- dispatch scopes
- feature flags
- derive stats
- time measurement units in metric kind (us, ms, etc.) for naming & scaling
- heartbeat metric on publish
- logger templates
- configurable aggregation
- non-aggregating buffers
- framework glue (rocket, iron, gotham, indicatif, etc.)
- more tests & benchmarks
- complete doc / inline samples
- more example apps
- A cool logo 
- method annotation processors `#[timer("name")]`
- fastsinks (M / &M) vs. safesinks (Arc<M>) 
