//! Queue metrics for write on a separate thread,
//! Metrics definitions and writers are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.

// TODO option to drop metrics when queue full

use core::*;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/////////////////////
// QUEUE

/// Thread safe sender to the queue
pub type QueueSender<M, W> = mpsc::SyncSender<QueueCommand<M, W>>;

struct QueueCommand<M, W> {
    /// The metric to write
    metric: Arc<M>,
    /// The writer to write the metric to
    writer: Arc<W>,
    /// The metric's reported value
    value : Value,
    /// The instant of measurement, if available
    time: Option<TimeHandle>,
}

/////////////////
// SINK

/// The queue writer simply sends the metric, writer and value over the channel for the actual write
/// to be performed synchronously by the queue command execution thread.
pub struct QueueWriter<M, W> {
    target_writer: Arc<W>,
    sender: QueueSender<M, W>,
}

impl <M, W> Writer<Arc<M>> for QueueWriter<M, W> where W: Writer<M> {
    fn write(&self, metric: &Arc<M>, value: Value) {
        self.sender.send(QueueCommand {
            metric: metric.clone(),
            writer: self.target_writer.clone(),
            value,
            time: Some(TimeHandle::now()),
        }).unwrap_or_else(|e| {/* TODO record error in selfstats */} )
    }
}

/// A metric command-queue using a sync channel.
/// Each client thread gets it's own writer / sender.
/// Writes are dispatched by a single receiving thread.
pub struct MetricQueue<M, W, S> {
    target: S,
    sender: QueueSender<M, W>,
}

impl<M, W, S> MetricQueue<M, W, S> where M: 'static + Send + Sync, W: 'static + Writer<M> + Send + Sync, S: Sink<M, W> {

    /// Build a new metric queue for asynchronous metric dispatch.
    pub fn new(target: S, queue_size: usize) -> MetricQueue<M, W, S> {
        let (sender, receiver) = mpsc::sync_channel::<QueueCommand<M, W>>(queue_size);
        thread::spawn(move || loop {
            while let Ok(cmd) = receiver.recv() {
                cmd.writer.write(&cmd.metric, cmd.value);
            }
        });
        MetricQueue { target, sender }
    }
}

impl<M, W, S> Sink<Arc<M>, QueueWriter<M, W>> for MetricQueue<M, W, S> where W: Writer<M>, S: Sink<M, W> {
    #[allow(unused_variables)]
    fn new_metric<STR: AsRef<str>>(&self, kind: MetricKind, name: STR, sampling: Rate) -> Arc<M> {
        Arc::new(self.target.new_metric(kind, name, sampling))
    }

    fn new_writer(&self) -> QueueWriter<M, W> {
        QueueWriter {
            target_writer: Arc::new(self.target.new_writer()),
            sender: self.sender.clone()
        }
    }
}
