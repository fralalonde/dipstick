//! Write metrics to Generic

use core::*;

pub fn print() -> FnSink<String> {
    make_sink(|k, n, r| format!("{:?} {} {}", k, n, r),
              |cmd| if let Some((m, v)) = cmd {println!("{}: {}", m, v)})
}

pub fn log<STR: AsRef<str> + 'static>(prefix: STR) -> FnSink<String> {
    make_sink(move |k, n, r| format!("{}{:?} {} {}", prefix.as_ref(), k, n, r),
              |cmd| if let Some((m, v)) = cmd {info!("{}: {}", m, v)})
}

pub fn make_sink<M, MF, WF  >(make_metric: MF, make_scope: WF) -> FnSink<M>
    where MF: Fn(Kind, &str, Rate) -> M + 'static,
          WF: Fn(Option<(&M, Value)>) + 'static,
{
    FnSink {
        metric_fn: Box::new(make_metric),
        scope_fn: Box::new(make_scope),
    }
}

pub struct FnSink<M> {
    metric_fn: Box<Fn(Kind, &str, Rate) -> M>,
    scope_fn: Box<Fn(Option<(&M, Value)>)>,
}

impl <M> Sink<M> for FnSink<M> {
    #[allow(unused_variables)]
    fn new_metric<STR>(&self, kind: Kind, name: STR, sampling: Rate) -> M
        where STR: AsRef<str>
    {
        self.metric_fn.as_ref()(kind, name.as_ref(), sampling)
    }

    fn new_scope(&self) -> &Fn(Option<(&M, Value)>) {
        &*self.scope_fn
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