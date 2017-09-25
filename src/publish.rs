//! Publishes metrics values from a source to a sink.
//!
//! ```
//! use dipstick::*;
//!
//! let (sink, source) = aggregate();
//! let publisher = publish(source, log("aggregated"));
//!
//! publisher.publish()
//! ```

use core::*;
use aggregate::{AggregateSource, ScoresSnapshot};
use std::time::Duration;
use std::thread;
use std::sync::atomic::AtomicUsize;

fn schedule<F>(every: Duration, operation: F)
    where F: Fn() -> () + Send + 'static
{
    thread::spawn(move || {
        loop {
            thread::sleep(every);
            // TODO add cancel
            operation();
        }
    });
}

/// Schedules the publisher to run at recurrent intervals
pub fn publish_every<M, S>(duration: Duration, source: AggregateSource, target: S)
    where S: Sink<M> + 'static + Send + Sync, M: Send + Sync
{
    schedule(duration, move || publish(&source, &target))
}

/// Define and write metrics from aggregated scores to the target channel
/// If this is called repeatedly it can be a good idea to use the metric cache
/// to prevent new metrics from being created every time.
pub fn publish<M, S>(source: &AggregateSource, target: &S)
    where S: Sink<M>, M: Send + Sync {
    let scope = target.new_scope();
    source.for_each(|metric| {
        match metric.read_and_reset() {
            ScoresSnapshot::NoData => {
                // TODO repeat previous frame min/max ?
                // TODO update some canary metric ?
            }
            ScoresSnapshot::Event { hit } => {
                let name = format!("{}.hit", &metric.name);
                let temp_metric = target.new_metric(Kind::Count, &name, 1.0);
                scope(Scope::Write(&temp_metric, hit));
            }
            ScoresSnapshot::Value { hit, sum, max, min } => {
                if hit > 0 {
                    match &metric.kind {
                        &Kind::Count |
                        &Kind::Time |
                        &Kind::Gauge => {
                            // NOTE best-effort averaging
                            // - hit and sum are not incremented nor read as one
                            // - integer division is not rounding
                            // assuming values will still be good enough to be useful
                            let name = format!("{}.avg", &metric.name);
                            let temp_metric = target.new_metric(metric.kind, &name, 1.0);
                            scope(Scope::Write(&temp_metric, sum / hit));
                        }
                        _ => (),
                    }

                    match &metric.kind {
                        // do not report gauges sum and hit, they are meaningless
                        &Kind::Count |
                        &Kind::Time => {
                            let name = format!("{}.sum", &metric.name);
                            let temp_metric = target.new_metric(metric.kind, &name, 1.0);
                            scope(Scope::Write(&temp_metric, sum));

                            let name = format!("{}.hit", &metric.name);
                            let temp_metric = target.new_metric(metric.kind, &name, 1.0);
                            scope(Scope::Write(&temp_metric, hit));
                        }
                        _ => (),
                    }

                    let name = format!("{}.max", &metric.name);
                    let temp_metric = target.new_metric(Kind::Gauge, &name, 1.0);
                    scope(Scope::Write(&temp_metric, max));

                    let name = format!("{}.min", &metric.name);
                    let temp_metric = target.new_metric(Kind::Gauge, &name, 1.0);
                    scope(Scope::Write(&temp_metric, min));
                }
            }
        }
    })
}
