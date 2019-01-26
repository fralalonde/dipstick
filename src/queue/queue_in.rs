//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.

use core::attributes::{Attributes, WithAttributes, Prefixed};
use core::name::MetricName;
use core::input::{InputKind, Input, InputScope, InputDyn, InputMetric};
use core::{MetricValue, Flush};
use core::metrics;
use cache::cache_in::CachedInput;
use core::error;
use core::label::Labels;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/// Wrap this output behind an asynchronous metrics dispatch queue.
/// This is not strictly required for multi threading since the provided scopes
/// are already Send + Sync but might be desired to lower the latency
pub trait QueuedInput: Input + Send + Sync + 'static + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn queued(self, max_size: usize) -> InputQueue {
        InputQueue::new(self, max_size)
    }
}

/// # Panics
///
/// Panics if the OS fails to create a thread.
fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<InputQueueCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<InputQueueCmd>(length);

    thread::Builder::new()
        .name("dipstick-queue-in".to_string())
        .spawn(move || {
            let mut done = false;
            while !done {
                match receiver.recv() {
                    Ok(InputQueueCmd::Write(metric, value, labels)) => metric.write(value, labels),
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
        })
        .unwrap(); // TODO: Panic, change API to return Result?
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
    pub fn new<OUT: Input + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        InputQueue {
            attributes: Attributes::default(),
            target: Arc::new(target),
            sender: new_async_channel(queue_length),
        }
    }
}

impl CachedInput for InputQueue {}

impl WithAttributes for InputQueue {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Input for InputQueue {
    type SCOPE = InputQueueScope;

    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    fn metrics(&self) -> Self::SCOPE {
        let target_scope = self.target.input_dyn();
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
    /// Send metric write
    Write(InputMetric, MetricValue, Labels),
    /// Send metric flush
    Flush(Arc<InputScope + Send + Sync + 'static>),
}

/// A metric scope wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct InputQueueScope {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<InputQueueCmd>>,
    target: Arc<InputScope + Send + Sync + 'static>,
}

impl InputQueueScope {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn wrap<SC: InputScope + Send + Sync + 'static>(target_scope: SC, queue_length: usize) -> Self {
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

impl InputScope for InputQueueScope {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        let target_metric = self.target.new_metric(name, kind);
        let sender = self.sender.clone();
        InputMetric::new(move |value, mut labels| {
            labels.save_context();
            if let Err(e) = sender.send(InputQueueCmd::Write(target_metric.clone(), value, labels)) {
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

