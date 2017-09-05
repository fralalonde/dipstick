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

#[derive(Debug)]
pub struct QueuedKey<C: MetricSink> {
    index: usize,
}

impl<C: MetricSink> MetricKey for QueuedKey<C> {}


#[derive(Debug)]
pub struct QueuedWriter<C: MetricSink> {
    sender: Arc<mpsc::SyncSender<QueuedWrite<C>>>,
}

impl<C: MetricSink> MetricWriter<QueuedKey<C>> for QueuedWriter<C> {
    fn write(&self, metric: &QueuedKey<C>, value: Value) {
        self.sender.send(QueuedWrite { metric, value, time: TimeHandle::now() })
    }
}

struct QueuedWrite<C: MetricSink> {
    metric: QueuedKey<C>,
    value : Value,
    time: TimeHandle
}

pub struct MetricQueue<C: MetricSink> {
    target: C,
    sender: Arc<mpsc::SyncSender<QueuedWrite<C>>>,
    target_metrics: Vec<Option<C::Metric>>,
}

impl<C: MetricSink> fmt::Debug for MetricQueue<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(self.target.fmt(f)?)
    }
}

impl<C: MetricSink> MetricQueue<C> {
    pub fn new(target: C, queue_size: usize) -> MetricQueue<C> {
        let (sender, receiver) = mpsc::sync_channel::<QueuedWrite<C>>(queue_size);
        let target_writer = target.new_writer();
        std::thread::spawn(move|| loop {
            while let Ok(qw) = receiver.recv() {
                target_writer.write(qw.metric.0, qw.value)
            }
        });
        MetricQueue { target, sender, target_metrics: Vec::new() }
    }
}

impl<C: MetricSink> MetricSink for MetricQueue<C> {
    type Metric = QueuedKey<C>;
    type Writer = QueuedWriter<C>;

    #[allow(unused_variables)]
    fn new_metric<S>(&self, kind: MetricKind, name: S, sampling: Rate) -> Self::Metric
            where S: AsRef<str>    {
        QueuedKey (self.target.new_metric(kind, name, sampling))
    }

    fn new_writer(&self) -> Self::Writer {
        QueuedWriter { sender: self.sender.clone() }
    }
}
