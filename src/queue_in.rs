//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
use core::{Scope, Value, InputMetric, Name, Kind, AddPrefix, InputDyn, Input,
           WithAttributes, Attributes, WithMetricCache, Flush};
use error;
use metrics;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<InputQueueCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<InputQueueCmd>(length);
    thread::spawn(move || {
        let mut done = false;
        while !done {
            match receiver.recv() {
                Ok(InputQueueCmd::Write(metric, value)) => metric.write(value),
                Ok(InputQueueCmd::Flush(scope)) => if let Err(e) = scope.flush() {
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
pub struct InputQueue {
    attributes: Attributes,
    target: Arc<InputDyn + Send + Sync + 'static>,
    sender: Arc<mpsc::SyncSender<InputQueueCmd>>,
}

impl InputQueue {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn new<OUT: InputDyn + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        InputQueue {
            attributes: Attributes::default(),
            target: Arc::new(target),
            sender: new_async_channel(queue_length),
        }
    }
}

impl WithMetricCache for InputQueue {}

impl WithAttributes for InputQueue {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Input for InputQueue {
    type SCOPE = InputQueueScope;

    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    fn open_scope(&self) -> Self::SCOPE {
        let target_scope = self.target.open_scope_dyn();
        InputQueueScope {
            attributes: self.attributes.clone(),
            sender: self.sender.clone(),
            target: target_scope,
        }
    }
}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum InputQueueCmd {
    Write(InputMetric, Value),
    Flush(Arc<Scope + Send + Sync + 'static>),
}

/// A metric scope wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct InputQueueScope {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<InputQueueCmd>>,
    target: Arc<Scope + Send + Sync + 'static>,
}

impl InputQueueScope {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn wrap<SC: Scope + Send + Sync + 'static>(target_scope: SC, queue_length: usize) -> Self {
        InputQueueScope {
            attributes: Attributes::default(),
            sender: new_async_channel(queue_length),
            target: Arc::new(target_scope),
        }
    }
}

impl WithAttributes for InputQueueScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Scope for InputQueueScope {
    fn new_metric(&self, name: Name, kind:Kind) -> InputMetric {
        let name = self.qualified_name(name);
        let target_metric = self.target.new_metric(name, kind);
        let sender = self.sender.clone();
        InputMetric::new(move |value| {
            if let Err(e) = sender.send(InputQueueCmd::Write(target_metric.clone(), value)) {
                metrics::SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for InputQueueScope {

    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(InputQueueCmd::Flush(self.target.clone())) {
            metrics::SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

