//! Dispatch metrics to multiple sinks.

use core::*;

/// Hold each sink's metric key.
pub type DoubleKey<M1, M2> = (M1, M2);

/// Hold the two target sinks.
/// Multiple `DoubleSink`s can be combined if more than two sinks are needed.
pub type DoubleSink<S1, S2> = (S1, S2);

impl<M1, S1, M2, S2> Sink<DoubleKey<M1, M2>> for DoubleSink<S1, S2>
    where S1: Sink<M1>, S2: Sink<M2>,
{
    #[allow(unused_variables)]
    fn new_metric<STR: AsRef<str>>(&self, kind: Kind, name: STR, sampling: Rate) -> DoubleKey<M1, M2> {
        (self.0.new_metric(kind, &name, sampling), self.1.new_metric(kind, &name, sampling))
    }

    fn new_scope(&self) -> &Fn(Option<(&DoubleKey<M1, M2>, Value)>) {
        let scope0 = self.0.new_scope();
        let scope1 = self.1.new_scope();
        &|cmd| {
            match cmd {
                Some((metric, value)) => {
                    scope0(Some((&metric.0, value)));
                    scope1(Some((&metric.1, value)));
                },
                None => {
                    scope0(None);
                    scope1(None);
                }
            }
        }
    }
}
