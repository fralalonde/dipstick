//! Queue metrics for write on a separate thread,
//! Metrics definitions and writers are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.

// TODO option to drop metrics when queue full

use ::*;
use cached::{SizedCache, Cached};
use std::sync::{Arc,RwLock};
use std::fmt;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

/////////////////////
// QUEUE

/// Thread safe sender to the queue
#[derive(Debug)]
pub struct QueueSender<C: Sink> (mpsc::SyncSender<QueueCommand<C>>);

struct QueueCommand<C: Sink, M: C::Metric, W: C::Writer+Sync> {
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

#[derive(Debug, Clone)]
pub struct QueueKey<C: Sink> (Arc<C::Metric>);

impl <C> Metric for QueueKey<C> where C: Sink {}

#[derive(Debug, Clone)]
pub struct QueueWriter<C: Sink> {
    target_writer: Arc<C::Writer>,
    sender: QueueSender<C>,
}

impl <C: Sink + Sync> Writer<QueueKey<C>> for QueueWriter<C> {
    fn write(&self, metric: &QueueKey<C>, value: Value) {
        self.sender.0.send(QueueCommand {
            metric: metric.clone(),
            writer: self.target_writer.clone(),
            value,
            time: Some(TimeHandle::now()),
        })
    }
}

/// A metric command-queue using a sync channel.
/// Each client thread gets it's own writer / sender.
/// Writes are dispatched by a single receiving thread.
pub struct MetricQueue<C: Sink> {
    target: C,
    sender: QueueSender<C>,
}

impl<C: Sink> fmt::Debug for MetricQueue<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(self.target.fmt(f)?)
    }
}

impl<C: Sink> MetricQueue<C> {
    pub fn new(target: C, queue_size: usize) -> MetricQueue<C> {
        let (sender, receiver) = mpsc::sync_channel::<QueueCommand<C>>(queue_size);
        let target_writer = target.new_writer();
        thread::spawn(move|| loop {
            while let Ok(cmd) = receiver.recv() {
                cmd.writer.write(cmd.0.metric, cmd.0.value);
            }
        });
        MetricQueue { target, sender: Arc::new(QueueSender(sender)) }
    }
}

impl<C: Sink> Sink for MetricQueue<C> {
    type Metric = QueueKey<C>;
    type Writer = QueueWriter<C>;

    #[allow(unused_variables)]
    fn new_metric<S>(&self, kind: MetricKind, name: S, sampling: Rate) -> Self::Metric where S: AsRef<str> {
        QueueKey(Arc::new(self.target.new_metric(kind, name, sampling)))
    }

    fn new_writer(&self) -> Self::Writer {
        QueueWriter {
            target_writer: Arc::new(self.target.new_writer()),
            sender: self.sender.clone()
        }
    }
}
