//! Write metrics to Generic

use core::*;
use std::sync::Arc;

pub fn print() -> FnSink<String> {
    make_sink(|k, n, r| format!("{:?} {} {}", k, n, r),
              |cmd| if let Some((m, v)) = cmd {println!("{}: {}", m, v)})
}

pub fn log<STR: AsRef<str> + 'static + Send + Sync>(prefix: STR) -> FnSink<String> {
    make_sink(move |k, n, r| format!("{}{:?} {} {}", prefix.as_ref(), k, n, r),
              |cmd| if let Some((m, v)) = cmd {info!("{}: {}", m, v)})
}

pub fn make_sink<M, MF, WF  >(make_metric: MF, make_scope: WF) -> FnSink<M>
    where MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
          WF: Fn(Option<(&M, Value)>) + Send + Sync + 'static,
          M: Send + Sync,
{
    FnSink {
        metric_fn: Arc::new(make_metric),
        scope_fn: Arc::new(make_scope),
    }
}

pub struct FnSink<M> where M: Send + Sync  {
    metric_fn: MetricFn<M>,
    scope_fn: ScopeFn<M>,
}

impl <M> Sink<M> for FnSink<M> where M: Send + Sync {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        self.metric_fn.as_ref()(kind, name, sampling)
    }

    fn new_scope(&self) -> ScopeFn<M> {
        self.scope_fn.clone()
    }
}

mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::print();
        let m = c.new_metric(Kind::Event, "test", 1.0);
        c.new_scope()(&m, 33);
    }

    #[test]
    fn log_print() {
        let c = super::log("log prefix");
        let m = c.new_metric(Kind::Event, "test", 1.0);
        c.new_scope()(&m, 33);
    }

}