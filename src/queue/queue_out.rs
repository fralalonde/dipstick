//! Queue metrics for write on a separate thread,
//! RawMetrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
//!
use core::attributes::{Attributes, WithAttributes, Naming};
use core::name::Name;
use core::input::{Kind, Input, InputScope, InputMetric};
use core::output::{OutputDyn, OutputScope, OutputMetric, Output};
use core::{Value, Flush};
use core::metrics;
use cache::cache_in;
use core::error;

use std::rc::Rc;
use std::ops;
use std::fmt;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/// Wrap this raw output behind an asynchronous metrics dispatch queue.
pub trait QueuedOutput: Output + Sized {
    /// Wrap this output with an asynchronous dispatch queue.
    fn queued(self, max_size: usize) -> OutputQueue {
        OutputQueue::new(self, max_size)
    }
}

fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<OutputQueueCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<OutputQueueCmd>(length);
    thread::spawn(move || {
        let mut done = false;
        while !done {
            match receiver.recv() {
                Ok(OutputQueueCmd::Write(metric, value)) => metric.write(value),
                Ok(OutputQueueCmd::Flush(scope)) => if let Err(e) = scope.flush() {
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
pub struct OutputQueue {
    attributes: Attributes,
    target: Arc<OutputDyn + Send + Sync + 'static>,
    q_sender: Arc<mpsc::SyncSender<OutputQueueCmd>>,
}

impl OutputQueue {
    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    pub fn new<OUT: Output + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        OutputQueue {
            attributes: Attributes::default(),
            target: Arc::new(target),
            q_sender: new_async_channel(queue_length),
        }
    }
}

impl WithAttributes for OutputQueue {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl cache_in::CachedInput for OutputQueue {}

impl Input for OutputQueue {
    type SCOPE = OutputQueueScope;

    /// Wrap new scopes with an asynchronous metric write & flush dispatcher.
    fn input(&self) -> Self::SCOPE {
        let target_scope = UnsafeScope::new(self.target.output_dyn());
        OutputQueueScope {
            attributes: self.attributes.clone(),
            sender: self.q_sender.clone(),
            target: Arc::new(target_scope),
        }
    }

}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum OutputQueueCmd {
    /// Send metric write
    Write(Arc<OutputMetric>, Value),
    /// Send metric flush
    Flush(Arc<UnsafeScope>),
}

/// A scope wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct OutputQueueScope {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<OutputQueueCmd>>,
    target: Arc<UnsafeScope>,
}

impl WithAttributes for OutputQueueScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl InputScope for OutputQueueScope {
    fn new_metric(&self, name: Name, kind:Kind) -> InputMetric {
        let name = self.naming_append(name);
        let target_metric = Arc::new(self.target.new_metric(name, kind));
        let sender = self.sender.clone();
        InputMetric::new(move |value| {
            if let Err(e) = sender.send(OutputQueueCmd::Write(target_metric.clone(), value)) {
                metrics::SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for OutputQueueScope {

    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(OutputQueueCmd::Flush(self.target.clone())) {
            metrics::SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

/// Wrap an OutputScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread or dragons may occur.
#[derive(Clone)]
pub struct UnsafeScope(Rc<OutputScope + 'static> );

unsafe impl Send for UnsafeScope {}
unsafe impl Sync for UnsafeScope {}

impl UnsafeScope {
    /// Wrap a dynamic RawScope to make it Send + Sync.
    pub fn new(scope: Rc<OutputScope + 'static>) -> Self {
        UnsafeScope(scope)
    }
}

impl ops::Deref for UnsafeScope {
    type Target = OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}


impl fmt::Debug for OutputMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Box<Fn(Value)>")
    }
}

unsafe impl Send for OutputMetric {}
unsafe impl Sync for OutputMetric {}

