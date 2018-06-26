//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
use core::{Scope, Value, Metric, Name, Kind, AddPrefix, OutputDyn, Output,
           WithAttributes, Attributes, WithMetricCache, Flush};
use error;
use metrics;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<QueueCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<QueueCmd>(length);
    thread::spawn(move || {
        let mut done = false;
        while !done {
            match receiver.recv() {
                Ok(QueueCmd::Write(metric, value)) => metric.write(value),
                Ok(QueueCmd::Flush(scope)) => if let Err(e) = scope.flush() {
                    debug!("Could not asynchronously flush metrics: {}", e);
                },
                Err(e) => {
                    debug!("Async metrics receive loop terminated: {}", e);
                    // cannot break from within match, use safety pin instead
                    done = true
                }
            }
        }
    });
    Arc::new(sender)
}

/// Wrap new scopes with an asynchronous metric write & flush dispatcher.
#[derive(Clone)]
pub struct QueueOutput {
    attributes: Attributes,
    target: Arc<OutputDyn + Send + Sync + 'static>,
    sender: Arc<mpsc::SyncSender<QueueCmd>>,
}

impl QueueOutput {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn new<OUT: OutputDyn + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        QueueOutput {
            attributes: Attributes::default(),
            target: Arc::new(target),
            sender: new_async_channel(queue_length),
        }
    }
}

impl WithMetricCache for QueueOutput {}

impl WithAttributes for QueueOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Output for QueueOutput {
    type SCOPE = Queue;

    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    fn open_scope(&self) -> Self::SCOPE {
        let target_scope = self.target.open_scope_dyn();
        Queue {
            attributes: self.attributes.clone(),
            sender: self.sender.clone(),
            target: target_scope,
        }
    }
}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum QueueCmd {
    Write(Metric, Value),
    Flush(Arc<Scope + Send + Sync + 'static>),
}

/// A metric scope wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct Queue {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<QueueCmd>>,
    target: Arc<Scope + Send + Sync + 'static>,
}

impl WithAttributes for Queue {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Scope for Queue {
    fn new_metric(&self, name: Name, kind:Kind) -> Metric {
        let name = self.qualified_name(name);
        let target_metric = self.target.new_metric(name, kind);
        let sender = self.sender.clone();
        Metric::new(move |value| {
            if let Err(e) = sender.send(QueueCmd::Write(target_metric.clone(), value)) {
                metrics::SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for Queue {

    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(QueueCmd::Flush(self.target.clone())) {
            metrics::SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

