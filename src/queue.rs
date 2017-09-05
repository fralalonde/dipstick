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

pub type QueuedKey<C: MetricSink> = Arc<C::Metric>;

impl <C> MetricKey for Arc<C> where C: MetricSink {}

pub type QueuedSender<C: MetricSink> = mpsc::SyncSender<QueuedWrite<C>>;

unsafe impl <C> Sync for mpsc::SyncSender<QueuedWrite<C>> where C: MetricSink {}

#[derive(Debug)]
pub struct QueuedWriter<C: MetricSink> {
    target_writer: Arc<C::Writer>,
    sender: Arc<QueuedSender<C>>,
}

impl<C: MetricSink> MetricWriter<QueuedKey<C>> for QueuedWriter<C> {
    fn write(&self, metric: &QueuedKey<C>, value: Value) {
        self.sender.send(QueuedWrite {
            metric: metric.clone(),
            writer: self.target_writer.clone(),
            value,
            time: Some(TimeHandle::now()),
        })
    }
}

struct QueuedWrite<C: MetricSink> {
    metric: Arc<C::Metric>,
    writer: Arc<C::Writer>,
    value : Value,
    time: Option<TimeHandle>,
}

pub struct MetricQueue<C: MetricSink> {
    target: C,
    sender: Arc<QueuedSender<C>>,
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
            while let Ok(cmd) = receiver.recv() {
                cmd.writer.write(cmd.metric, cmd.value);
            }
        });
        MetricQueue { target, sender: Arc::new(sender) }
    }
}

impl<C: MetricSink> MetricSink for MetricQueue<C> {
    type Metric = QueuedKey<C>;
    type Writer = QueuedWriter<C>;

    #[allow(unused_variables)]
    fn new_metric<S>(&self, kind: MetricKind, name: S, sampling: Rate) -> Self::Metric
            where S: AsRef<str>    {
        Arc::new(self.target.new_metric(kind, name, sampling))
    }

    fn new_writer(&self) -> Self::Writer {
        QueuedWriter {
            target_writer: Arc::new(self.target.new_writer()),
            sender: self.sender.clone()
        }
    }
}
