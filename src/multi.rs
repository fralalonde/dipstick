//! Dispatch metrics to multiple sinks.

use core::*;
use std::sync::Arc;

/// Hold each sink's metric key.
pub type DoubleKey<M1, M2> = (M1, M2);

/// Hold the two target sinks.
/// Multiple `DoubleSink`s can be combined if more than two sinks are needed.
pub type DoubleSink<S1, S2> = (S1, S2);

impl<M1, S1, M2, S2> Sink<DoubleKey<M1, M2>> for DoubleSink<S1, S2>
    where S1: Sink<M1>, S2: Sink<M2>, M1: 'static + Send + Sync, M2: 'static + Send + Sync
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> DoubleKey<M1, M2> {
        (self.0.new_metric(kind, name, sampling), self.1.new_metric(kind, &name, sampling))
    }

    fn new_scope(&self) -> ScopeFn<DoubleKey<M1, M2>> {
        let scope0 = self.0.new_scope();
        let scope1 = self.1.new_scope();
        Arc::new(move |cmd| {
            match cmd {
                Scope::Write(metric, value) => {
                    scope0(Scope::Write(&metric.0, value));
                    scope1(Scope::Write(&metric.1, value));
                },
                Scope::Flush => {
                    scope0(Scope::Flush);
                    scope1(Scope::Flush);
                }
            }
        })
    }
}
