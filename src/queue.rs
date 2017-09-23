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
pub fn queue<M, S, WF>(queue_size: usize, sink: S) -> MetricQueue<M, S, WF>
    where M: 'static + Send + Sync, S: Sink<M>,
          WF: Fn(Option<(&M, Value)>) + Send + Sync
{
    let (sender, receiver) = mpsc::sync_channel::<QueueCommand<M, WF>>(queue_size);
    thread::spawn(move || loop {
        while let Ok(cmd) = receiver.recv() {
            // apply scope commands received from channel
            match cmd {
                QueueCommand {cmd: Some((metric, value)), next_scope, .. } => next_scope(Some((metric.as_ref(), value))),
                QueueCommand {cmd: None, next_scope, .. } => next_scope(None),
            }
        }
    });
    MetricQueue { next_sink: sink, sender }
}

/////////////////////
// QUEUE

/// Thread safe sender to the queue
pub type QueueSender<M, WF> = mpsc::SyncSender<QueueCommand<M, WF>>;

struct QueueCommand<M, WF>
    where WF: Fn(Option<(&M, Value)>) + Send + Sync
{
    /// The metric and value to write
    cmd: Option<(Arc<M>, Value)>,
    /// The scope to write the metric to
    next_scope: Arc<WF>,
}

/// A metric command-queue using a sync channel.
/// Each client thread gets it's own scope and sender.
/// Writes are dispatched by a single receiving thread.
pub struct MetricQueue<M, S, WF>
    where WF: Fn(Option<(&M, Value)>) + Send + Sync + 'static
{
    next_sink: S,
    sender: QueueSender<M, WF>,
}

impl<M, S, WF> Sink<Arc<M>> for MetricQueue<M, S, WF>
    where S: Sink<M>, WF: Fn(Option<(&M, Value)>) + Send + Sync
{
    #[allow(unused_variables)]
    fn new_metric<STR: AsRef<str>>(&self, kind: Kind, name: STR, sampling: Rate) -> Arc<M> {
        Arc::new(self.next_sink.new_metric(kind, name, sampling))
    }

    fn new_scope(&self) -> Box<Fn(Option<(&Arc<M>, Value)>)> {
        // open next scope
        let next_scope: Arc<WF> = Arc::new(self.next_sink.new_scope());
        // forward any scope command through the channel
        Box::new(|cmd| {
            let send_cmd = match cmd {
                Some((metric, value)) => Some(((*metric).clone(), value)),
                None => None,
            };
            self.sender.send(QueueCommand {
                cmd: send_cmd,
                next_scope: next_scope2.clone(),
            }).unwrap_or_else(|e| { /* TODO dropping queue command, record fault in selfstats */})
        })
    }
}
