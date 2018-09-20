# aggregation

## bucket

Aggregation is performed locklessly and is very fast.
Count, sum, min, max and average are tracked where they make sense.

## schedule

Aggregate metrics and schedule to be periodical publication in the background:
```rust,skt-run
use std::time::Duration;

let app_metrics = metric_scope(aggregate());
route_aggregate_metrics(to_stdout());
app_metrics.flush_every(Duration::from_secs(3));
```

## preset statistics

Published statistics can be selected with presets such as `all_stats` (see previous example),
`summary`, `average`.


## custom statistics

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
