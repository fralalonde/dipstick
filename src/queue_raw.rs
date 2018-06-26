//! Queue metrics for write on a separate thread,
//! RawMetrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
use core::{Value, RawMetric, Name, Kind, AddPrefix, RawOutputDyn,
           WithAttributes, Attributes, Scope, Output, Metric, UnsafeScope, Flush};
use error;
use metrics;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<RawQueueCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<RawQueueCmd>(length);
    thread::spawn(move || {
        let mut done = false;
        while !done {
            match receiver.recv() {
                Ok(RawQueueCmd::Write(metric, value)) => metric.write(value),
                Ok(RawQueueCmd::Flush(scope)) => if let Err(e) = scope.flush() {
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


/// Wrap scope with an asynchronous metric write & flush dispatcher.
#[derive(Clone)]
pub struct RawQueueOutput {
    attributes: Attributes,
    target: Arc<RawOutputDyn + Send + Sync + 'static>,
    sender: Arc<mpsc::SyncSender<RawQueueCmd>>,
}

impl RawQueueOutput {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn new<OUT: RawOutputDyn + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        RawQueueOutput {
            attributes: Attributes::default(),
            target: Arc::new(target),
            sender: new_async_channel(queue_length),
        }
    }
}

impl WithAttributes for RawQueueOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Output for RawQueueOutput {
    type SCOPE = RawQueue;

    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    fn open_scope(&self) -> Self::SCOPE {
        let target_scope = UnsafeScope::new(self.target.open_scope_raw_dyn());
        RawQueue {
            attributes: self.attributes.clone(),
            sender: self.sender.clone(),
            target: Arc::new(target_scope),
        }
    }

}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum RawQueueCmd {
    Write(Arc<RawMetric>, Value),
    Flush(Arc<UnsafeScope>),
}

/// A scope wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct RawQueue {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<RawQueueCmd>>,
    target: Arc<UnsafeScope>,
}

impl WithAttributes for RawQueue {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Scope for RawQueue {
    fn new_metric(&self, name: Name, kind:Kind) -> Metric {
        let name = self.qualified_name(name);
        let target_metric = Arc::new(self.target.new_metric_raw(name, kind));
        let sender = self.sender.clone();
        Metric::new(move |value| {
            if let Err(e) = sender.send(RawQueueCmd::Write(target_metric.clone(), value)) {
                metrics::SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for RawQueue {

    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(RawQueueCmd::Flush(self.target.clone())) {
            metrics::SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}
