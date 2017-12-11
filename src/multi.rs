//! Dispatch metrics to multiple sinks.

use core::*;
use std::sync::Arc;

/// Multiple chain can be combined
impl<M1, M2> From<(Chain<M1>, Chain<M2>)> for Chain<(M1, M2)>
    where
        M1: 'static + Clone + Send + Sync,
        M2: 'static + Clone + Send + Sync,
{
    fn from(combo: (Chain<M1>, Chain<M2>)) -> Chain<(M1, M2)> {

        let combo0 = combo.0.clone();
        let combo1 = combo.1.clone();

        Chain::new(
            move |kind, name, sampling| (
                combo.0.define_metric(kind, name, sampling),
                combo.1.define_metric(kind, &name, sampling),
            ),

            move |auto_flush| {
                let scope0 = combo0.open_scope(auto_flush);
                let scope1 = combo1.open_scope(auto_flush);

                Arc::new(move |cmd| match cmd {
                    ScopeCmd::Write(metric, value) => {
                        scope0(ScopeCmd::Write(&metric.0, value));
                        scope1(ScopeCmd::Write(&metric.1, value));
                    }
                    ScopeCmd::Flush => {
                        scope0(ScopeCmd::Flush);
                        scope1(ScopeCmd::Flush);
                    }
                })
            },
        )
    }
}
