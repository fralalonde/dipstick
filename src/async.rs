//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
use core::{Input, Value, Metric, Name, Kind, Flush, Marker, WithName, OutputDyn, Output,
           WithAttributes, Attributes};

use bucket::Bucket;
use error;
use self_metrics::DIPSTICK_METRICS;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

metrics!{
    <Bucket> DIPSTICK_METRICS.add_name("async_queue") => {
        /// Maybe queue was full?
        Marker SEND_FAILED: "send_failed";
    }
}

fn new_async_channel(length: usize) -> Arc<mpsc::SyncSender<AsyncCmd>> {
    let (sender, receiver) = mpsc::sync_channel::<AsyncCmd>(length);
    thread::spawn(move || {
        let mut done = false;
        while !done {
            match receiver.recv() {
                Ok(AsyncCmd::Write(wfn, value)) => (wfn)(value),
                Ok(AsyncCmd::Flush(input)) => if let Err(e) = input.flush() {
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

/// Wrap new inputs with an asynchronous metric write & flush dispatcher.
#[derive(Clone)]
pub struct AsyncOutput {
    attributes: Attributes,
    target: Arc<OutputDyn + Send + Sync + 'static>,
    sender: Arc<mpsc::SyncSender<AsyncCmd>>,
}

impl AsyncOutput {
    /// Wrap new inputs with an asynchronous metric write & flush dispatcher.
    pub fn new<OUT: OutputDyn + Send + Sync + 'static>(target: OUT, queue_length: usize) -> Self {
        AsyncOutput {
            attributes: Attributes::default(),
            target: Arc::new(target),
            sender: new_async_channel(queue_length),
        }
    }
}

impl WithAttributes for AsyncOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Output for AsyncOutput {
    type INPUT = AsyncInput;

    /// Wrap new inputs with an asynchronous metric write & flush dispatcher.
    fn new_input(&self) -> Self::INPUT {
        let target_input = self.target.new_input_dyn();
        AsyncInput {
            attributes: self.attributes.clone(),
            sender: self.sender.clone(),
            target: target_input,
        }
    }

    fn async(self, _queue_length: usize) -> Self {
        warn!("Ignoring async on async");
        self
    }

}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum AsyncCmd {
    Write(Metric, Value),
    Flush(Arc<Input + Send + Sync + 'static>),
}

/// A metric input wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct AsyncInput {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<AsyncCmd>>,
    target: Arc<Input + Send + Sync + 'static>,
}

impl WithAttributes for AsyncInput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Input for AsyncInput {
    fn new_metric(&self, name: Name, kind:Kind) -> Metric {
        let name = self.qualified_name(name);
        let target_metric = self.target.new_metric(name, kind);
        let sender = self.sender.clone();
        Metric::new(move |value| {
            if let Err(e) = sender.send(AsyncCmd::Write(target_metric.clone(), value)) {
                SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for AsyncInput {
    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(AsyncCmd::Flush(self.target.clone())) {
            SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}
