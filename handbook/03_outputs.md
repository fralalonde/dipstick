# outputs

## statsd

## graphite

## text

## logging

## prometheus

## combination

Send metrics to multiple outputs:

```rust,skt-fail,no_run
let _app_metrics = metric_scope((
        to_stdout(), 
        to_statsd("localhost:8125")?.with_namespace(&["my", "app"])
    ));
```

## buffering

## sampling

Apply statistical sampling to metrics:

```rust,skt-fail
let _app_metrics = to_statsd("server:8125")?.with_sampling_rate(0.01);
```

A fast random algorithm (PCG32) is used to pick samples.
Outputs can use sample rate to expand or format published data.



