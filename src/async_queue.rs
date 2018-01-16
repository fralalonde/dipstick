//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
//!
use core::*;
use self_metrics::*;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

mod_metrics!(Aggregate, QUEUE_METRICS = DIPSTICK_METRICS.with_prefix("async_queue"));
mod_marker!(Aggregate, QUEUE_METRICS, { SEND_FAILED: "send_failed" });

/// Enqueue collected metrics for dispatch on background thread.
pub trait WithAsyncQueue
where
    Self: Sized,
{
    /// Enqueue collected metrics for dispatch on background thread.
    fn with_async_queue(&self, queue_size: usize) -> Self;
}

impl<M: Send + Sync + Clone + 'static> WithAsyncQueue for Chain<M> {
    fn with_async_queue(&self, queue_size: usize) -> Self {
        self.mod_scope(|next| {
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

            Arc::new(move |buffered| {
                // open next scope, make it Arc to move across queue
                let next_scope: ControlScopeFn<M> = next(buffered);
                let sender = sender.clone();

                // forward any scope command through the channel
                ControlScopeFn::new(move |cmd| {
                    let send_cmd = match cmd {
                        ScopeCmd::Write(metric, value) => {
                            let metric: &M = metric;
                            Some((metric.clone(), value))
                        },
                        ScopeCmd::Flush => None,
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

/// Enqueue collected metrics for dispatch on background thread.
#[deprecated(since = "0.5.0", note = "Use `with_async_queue` instead.")]
pub fn async<M, IC>(queue_size: usize, chain: IC) -> Chain<M>
where
    M: Clone + Send + Sync + 'static,
    IC: Into<Chain<M>>,
{
    let chain = chain.into();
    chain.with_async_queue(queue_size)
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
    next_scope: ControlScopeFn<M>,
}
