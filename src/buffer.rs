/*!
UNUSED FOR THE MOMENT
archived code
*/
use core::*;
use core::ScopeCmd::*;
use std::sync::{Arc, RwLock};

pub trait WithBuffer {
    fn with_buffered_scopes(&self) -> Self;
}

impl<M: Send + Sync + Clone + 'static> WithBuffer for Chain<M> {
    /// Create a new scope to report metric values.
    fn with_buffered_scopes(&self) -> Self {
        self.mod_scope(|next| {
            let scope_buffer = RwLock::new(ScopeBuffer {
                buffer: Vec::new(),
                scope: self.open_scope(false),
            });
            Arc::new(move |cmd: ScopeCmd<M>| {
                let mut buf = scope_buffer.write().expect("Lock metric scope");
                match cmd {
                    Write(metric, value) => buf.buffer.push(ScopeCommand {
                        metric: (*metric).clone(),
                        value,
                    }),
                    Flush => buf.flush(),
                }
            })
        })
    }

}

/// Save the metrics for delivery upon scope close.
struct ScopeCommand<M> {
    metric: M,
    value: Value,
}

struct ScopeBuffer<M: Clone> {
    buffer: Vec<ScopeCommand<M>>,
    scope: ControlScopeFn<M>,
}

impl<M: Clone> ScopeBuffer<M> {
    fn flush(&mut self) {
        for cmd in self.buffer.drain(..) {
            (self.scope)(Write(&cmd.metric, cmd.value))
        }
        (self.scope)(Flush)
    }
}
