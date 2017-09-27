//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
// TODO option to drop metrics when queue full

use core::*;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn queue<M, S>(queue_size: usize, sink: S) -> MetricQueue<M, S>
    where M: 'static + Send + Sync, S: Sink<M>
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

/// Thread safe sender to the queue
pub type QueueSender<M> = mpsc::SyncSender<QueueCommand<M>>;

/// Carry the scope command over the queue, from the sender, to be executed by the receiver.
pub struct QueueCommand<M> {
    /// If Some(), the metric and value to write.
    /// If None, flush the scope
    cmd: Option<(Arc<M>, Value)>,
    /// The scope to write the metric to
    next_scope: Arc<ScopeFn<M>>,
}

/// A metric command-queue using a sync channel.
/// Each client thread gets it's own scope and sender.
/// Writes are dispatched by a single receiving thread.
pub struct MetricQueue<M, S> {
    next_sink: S,
    sender: QueueSender<M>,
}

impl<M, S> Sink<Arc<M>> for MetricQueue<M, S> where S: Sink<M>, M: 'static + Send + Sync {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Arc<M> {
        Arc::new(self.next_sink.new_metric(kind, name, sampling))
    }

    fn new_scope(&self) -> ScopeFn<Arc<M>> {
        // open next scope, make it Arc to move across queue
        let next_scope: Arc<ScopeFn<M>> = Arc::from(self.next_sink.new_scope());

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
            }).unwrap_or_else(|e| { /* TODO dropping queue command, record fault in selfstats */})
        })
    }
}
