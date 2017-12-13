//! Dispatch metrics to multiple sinks.

use core::*;
use std::sync::Arc;

/// Two chains of different types can be combined in a tuple.
/// The chains will act as one, each receiving calls in the order the appear in the tuple.
/// For more than two types, make tuples of tuples, "Yo Dawg" style.
impl<M1, M2> From<(Chain<M1>, Chain<M2>)> for Chain<(M1, M2)>
    where
        M1: 'static + Clone + Send + Sync,
        M2: 'static + Clone + Send + Sync,
{
    fn from(combo: (Chain<M1>, Chain<M2>)) -> Chain<(M1, M2)> {

        let combo0 = combo.0.clone();
        let combo1 = combo.1.clone();

        Chain::new(
            move |kind, name, rate| (
                combo.0.define_metric(kind, name, rate),
                combo.1.define_metric(kind, &name, rate),
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

/// Multiple chains of the same type can be combined in a slice.
/// The chains will act as one, each receiving calls in the order the appear in the slice.
impl<'a, M> From<&'a [Chain<M>]> for Chain<Box<[M]>>
    where
        M: 'static + Clone + Send + Sync,
{
    fn from(chains: &'a [Chain<M>]) -> Chain<Box<[M]>> {

        let chains = chains.to_vec();
        let chains2 = chains.clone();

        Chain::new(
            move |kind, name, rate| {
                let mut metric = Vec::with_capacity(chains.len());
                for chain in &chains {
                    metric.push(chain.define_metric(kind, name, rate));
                }
                metric.into_boxed_slice()
            },

            move |auto_flush| {
                let mut scopes = Vec::with_capacity(chains2.len());
                for chain in &chains2 {
                    scopes.push(chain.open_scope(auto_flush));
                }

                Arc::new(move |cmd| match cmd {
                    ScopeCmd::Write(metric, value) => {
                        for (i, scope) in scopes.iter().enumerate() {
                            (scope)(ScopeCmd::Write(&metric[i], value))
                        }
                    }
                    ScopeCmd::Flush => {
                        for scope in &scopes {
                            (scope)(ScopeCmd::Flush)
                        }
                    }
                })
            },
        )
    }
}
