//! Queue metrics for write on a separate thread,
//! Metrics definitions are still synchronous.
//! If queue size is exceeded, calling code reverts to blocking.
//!
use core::*;
use self_metrics::*;

use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn async<M, IC>(queue_size: usize, chain: IC) -> Chain<M>
    where
        M: Clone + Send + Sync + 'static,
        IC: Into<Chain<M>>,
{
    let chain = chain.into();
    chain.mod_scope(|next| {
        // setup channel
        let (sender, receiver) = mpsc::sync_channel::<QueueCommand<M>>(queue_size);

        // start queue processor thread
        thread::spawn(move || loop {
            while let Ok(cmd) = receiver.recv() {
                match cmd {
                    QueueCommand { cmd: Some((metric, value)), next_scope } =>
                        next_scope(ScopeCmd::Write(&metric, value)),
                    QueueCommand { cmd: None, next_scope } =>
                        next_scope(ScopeCmd::Flush),
                }
            }
        });

        Arc::new(move |auto_flush| {
            // open next scope, make it Arc to move across queue
            let next_scope: Arc<ControlScopeFn<M>> = Arc::from(next(auto_flush));
            let sender = sender.clone();

            // forward any scope command through the channel
            Arc::new(move |cmd| {
                let send_cmd = match cmd {
                    ScopeCmd::Write(metric, value) => Some((metric.clone(), value)),
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

lazy_static! {
    static ref QUEUE_METRICS: GlobalMetrics<Aggregate> = SELF_METRICS.with_prefix("async.");
    static ref SEND_FAILED: Marker<Aggregate> = QUEUE_METRICS.marker("send_failed");
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
    next_scope: Arc<ControlScopeFn<M>>,
}

