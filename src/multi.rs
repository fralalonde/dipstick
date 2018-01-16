//! Dispatch metrics to multiple sinks.

use core::*;

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
            move |kind, name, rate| {
                (
                    combo.0.define_metric(kind, name, rate),
                    combo.1.define_metric(kind, &name, rate),
                )
            },
            move |buffered| {
                let scope0 = combo0.open_scope(buffered);
                let scope1 = combo1.open_scope(buffered);

                ControlScopeFn::new(move |cmd| match cmd {
                    ScopeCmd::Write(metric, value) => {
                        let metric: &(M1, M2) = metric;
                        scope0.write(&metric.0, value);
                        scope1.write(&metric.1, value);
                    }
                    ScopeCmd::Flush => {
                        scope0.flush();
                        scope1.flush();
                    }
                })
            },
        )
    }
}

/// Multiple chains of the same type can be combined in a slice.
/// The chains will act as one, each receiving calls in the order the appear in the slice.
impl<'a, M> From<&'a [Chain<M>]> for Chain<Vec<M>>
where
    M: 'static + Clone + Send + Sync,
{
    fn from(chains: &'a [Chain<M>]) -> Chain<Vec<M>> {
        let chains = chains.to_vec();
        let chains2 = chains.clone();

        Chain::new(
            move |kind, name, rate| {
                let mut metric = Vec::with_capacity(chains.len());
                for chain in &chains {
                    metric.push(chain.define_metric(kind, name, rate));
                }
                metric
            },
            move |buffered| {
                let mut scopes = Vec::with_capacity(chains2.len());
                for chain in &chains2 {
                    scopes.push(chain.open_scope(buffered));
                }

                ControlScopeFn::new(move |cmd| match cmd {
                    ScopeCmd::Write(metric, value) => {
                        let metric: &Vec<M> = metric;
                        for (i, scope) in scopes.iter().enumerate() {
                            scope.write(&metric[i], value)
                        }
                    },
                    ScopeCmd::Flush => for scope in &scopes {
                        scope.flush()
                    },
                })
            },
        )
    }
}
