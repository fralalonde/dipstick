//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
//!
use core::*;
use output::*;
use self_metrics::*;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

metrics!{
    Aggregate = DIPSTICK_METRICS.with_prefix("async_queue") => {
        /// Maybe queue was full?
        SEND_FAILED: Marker = "send_failed";
    }
}

/// Enqueue collected metrics for dispatch on background thread.
pub trait WithAsyncQueue
where
    Self: Sized,
{
    /// Enqueue collected metrics for dispatch on background thread.
    fn with_async_queue(&self, queue_size: usize) -> Self;
}

impl<M: Send + Sync + Clone + 'static> WithAsyncQueue for MetricOutput {
    fn with_async_queue(&self, queue_size: usize) -> Self {
        self.wrap_scope(|next| {
            // setup channel
            let (sender, receiver) = mpsc::sync_channel::<QueueCommand<M>>(queue_size);

            // start queue processor thread
            thread::spawn(move || loop {
                while let Ok(cmd) = receiver.recv() {
                    match cmd {
                        QueueCommand {
                            cmd: Some((metric, value)),
                            next_scope,
                        } => next_scope.write(&metric, value),
                        QueueCommand {
                            cmd: None,
                            next_scope,
                        } => next_scope.flush(),
                    }
                }
            });

            Arc::new(move || {
                // open next scope, make it Arc to move across queue
                let next_scope: CommandFn<M> = next();
                let sender = sender.clone();

                // forward any scope command through the channel
                command_fn(move |cmd| {
                    let send_cmd = match cmd {
                        Command::Write(metric, value) => {
                            let metric: &M = metric;
                            Some((metric.clone(), value))
                        }
                        Command::Flush => None,
                    };
                    sender
                        .send(QueueCommand {
                            cmd: send_cmd,
                            next_scope: next_scope.clone(),
                        })
                        .unwrap_or_else(|e| {
                            SEND_FAILED.mark();
                            trace!("Async metrics could not be sent: {}", e);
                        })
                })
            })
        })
    }
}

/// Carry the scope command over the queue, from the sender, to be executed by the receiver.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct QueueCommand<M> {
    /// If Some(), the metric and value to write.
    /// If None, flush the scope
    cmd: Option<(M, Value)>,
    /// The scope to write the metric to
    #[derivative(Debug = "ignore")]
    next_scope: CommandFn<M>,
}
