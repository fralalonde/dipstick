# concurrency concerns

## locking

## queueing

Metrics can be recorded asynchronously:
```rust,skt-run
let _app_metrics = metric_scope(to_stdout().with_async_queue(64));
```
The async queue uses a Rust channel and a standalone thread.
The current behavior is to block when full.
