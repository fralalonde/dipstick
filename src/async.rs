//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
//!
use core::{Input, Value, WriteFn, Name, Kind, Flush, Marker, WithName,
           Attributes, WithAttributes};
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

/// Wrap the input with an async dispatch queue for lower app latency.
pub fn to_async<IN: Input + Send + Sync + 'static + Clone>(input: IN, queue_length: usize) -> AsyncInput {
    AsyncInput::wrap(input, queue_length)
}

//pub fn to_async<OUT: MetricOutput + Send + Sync + 'static + Clone>(input: OUT, queue_length: usize) -> AsyncInput {
//    AsyncInput::wrap(input, queue_length)
//}

//pub struct AsyncOutput {
//    attributes: Attributes,
//    sender: Arc<mpsc::SyncSender<AsyncCmd>>,
//}

/// This is only `pub` because `error` module needs to know about it.
/// Async commands should be of no concerns to applications.
pub enum AsyncCmd {
    Write(WriteFn, Value),
    Flush,
}

/// A metric input wrapper that sends writes & flushes over a Rust sync channel.
/// Commands are executed by a background thread.
#[derive(Clone)]
pub struct AsyncInput {
    attributes: Attributes,
    sender: Arc<mpsc::SyncSender<AsyncCmd>>,
    input: Arc<Input + Send + Sync + 'static>
}

impl WithAttributes for AsyncInput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl AsyncInput {
    /// Wrap the input with an async dispatch queue for lower app latency.
    pub fn wrap(input: impl Input + Send + Sync + 'static + Clone, queue_length: usize) -> Self {
        let flusher = input.clone();
        let (sender, receiver) = mpsc::sync_channel::<AsyncCmd>(queue_length);
        thread::spawn(move || {
            let mut done = false;
            while !done {
                match receiver.recv() {
                    Ok(AsyncCmd::Write(wfn, value)) => (wfn)(value),
                    Ok(AsyncCmd::Flush) => if let Err(e) = flusher.flush() {
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
        AsyncInput {
            attributes: Attributes::default(),
            sender: Arc::new(sender),
            input: Arc::new(input),
        }
    }
}

impl Input for AsyncInput {
    fn new_metric(&self, name: Name, kind:Kind) -> WriteFn {
        let target_metric = self.input.new_metric(self.qualified_name(name), kind);
        let sender = self.sender.clone();
        WriteFn::new(move |value| {
            if let Err(e) = sender.send(AsyncCmd::Write(target_metric.clone(), value)) {
                SEND_FAILED.mark();
                debug!("Failed to send async metrics: {}", e);
            }
        })
    }
}

impl Flush for AsyncInput {
    fn flush(&self) -> error::Result<()> {
        if let Err(e) = self.sender.send(AsyncCmd::Flush) {
            SEND_FAILED.mark();
            debug!("Failed to flush async metrics: {}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}
