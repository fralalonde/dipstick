//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
// TODO option to drop metrics when queue full

use core::*;
use selfmetrics::*;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn async<M, S>(queue_size: usize, sink: S) -> MetricQueue<M, S>
    where S: Sink<M>,
          M: 'static + Clone + Send + Sync
{
    let (sender, receiver) = mpsc::sync_channel::<QueueCommand<M>>(queue_size);
    thread::spawn(move || loop {
        while let Ok(cmd) = receiver.recv() {
            // apply scope commands received from channel
            match cmd {
                QueueCommand {cmd: Some((metric, value)), next_scope, .. } => next_scope(Scope::Write(metric.as_ref(), value)),
                QueueCommand {cmd: None, next_scope, .. } => next_scope(Scope::Flush),
            }
        }
    });
    MetricQueue { next_sink: sink, sender }
}

lazy_static! {
    static ref QUEUE_METRICS: AppMetrics<Aggregate, AggregateSink> =
                                            SELF_METRICS.with_prefix("async.");

    static ref SEND_FAILED: Marker<Aggregate> = QUEUE_METRICS.marker("send_failed");
}

/// Thread safe sender to the queue
pub type QueueSender<M> = mpsc::SyncSender<QueueCommand<M>>;

/// Carry the scope command over the queue, from the sender, to be executed by the receiver.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct QueueCommand<M> {
    /// If Some(), the metric and value to write.
    /// If None, flush the scope
    cmd: Option<(Arc<M>, Value)>,
    /// The scope to write the metric to
    #[derivative(Debug="ignore")]
    next_scope: Arc<ScopeFn<M>>,
}

/// A metric command-queue using a sync channel.
/// Each client thread gets it's own scope and sender.
/// Writes are dispatched by a single receiving thread.
#[derive(Debug)]
pub struct MetricQueue<M, S> {
    next_sink: S,
    sender: QueueSender<M>,
}

impl<M, S> Sink<Arc<M>> for MetricQueue<M, S> where S: Sink<M>, M: 'static + Clone + Send + Sync {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Arc<M> {
        Arc::new(self.next_sink.new_metric(kind, name, sampling))
    }

    fn new_scope(&self, auto_flush: bool) -> ScopeFn<Arc<M>> {
        // open next scope, make it Arc to move across queue
        let next_scope: Arc<ScopeFn<M>> = Arc::from(self.next_sink.new_scope(auto_flush));

        let sender = self.sender.clone();

        // forward any scope command through the channel
        Arc::new(move |cmd| {
            let send_cmd = match cmd {
                Scope::Write(metric, value) => Some(((*metric).clone(), value)),
                Scope::Flush => None,
            };
            sender.send(QueueCommand {
                cmd: send_cmd,
                next_scope: next_scope.clone(),
            }).unwrap_or_else(|e| {
                SEND_FAILED.mark();
                trace!("Async metrics could not be sent: {}", e);
            })
        })
    }
}
