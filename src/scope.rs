pub use core::{MetricType, Rate, Value, MetricWriter, MetricKey, MetricSink};

#[derive(Debug)]
pub struct ScopeKey<M: MetricKey> {
    target: M,
}

impl<M: MetricKey> MetricKey for ScopeKey<M> {}

#[derive(Debug)]
pub struct ScopeWriter<C: MetricSink> {
    default_writer: C::Writer,
    thread_writer: ThreadLocal<C::Writer>,
}

impl<C: MetricSink> MetricWriter<ScopeKey<<C as MetricSink>::Metric>> for ScopeWriter<C> {
    fn write(&self, metric: &ScopeKey<<C as MetricSink>::Metric>, value: Value) {
        let scope = self.thread_writer.get(|scope| match scope {
            Some(scoped) => scoped.writer.write(metric, value),
            None => self.default_writer.write(metric, value),
        });
    }

}

//impl<C: MetricSink> DispatchScope for ScopeWriter<C> {
//    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
//        self
//    }
//}

//impl<C: MetricSink> DispatchScope for DirectDispatchWriter<C> {
//    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
//        self
//    }
//}

//    fn with_scope<F>(&mut self, operations: F)
//    where
//        F: Fn(&Self::Scope),
//    {
//        let new_writer = self.target.new_writer();
//        let scope = ScopeWriter { writer: new_writer };
//        // TODO add ThreadLocal with(T, FnOnce) method to replace these three
//        self.dispatch_scope.thread_scope.set(scope);
//        self.dispatch_scope.thread_scope.get(|option_scope| {
//            operations(option_scope.unwrap())
//        });
//        self.dispatch_scope.thread_scope.remove();
//    }


#[derive(Debug)]
pub struct ScopeSink<C: MetricSink> {
    target: C,
    writer: Arc<ScopeWriter<C>>,
}

impl<C: MetricSink> ScopeSink<C> {
    pub fn new(target: C, sampling_rate: Rate) -> ScopeSink<C> {
        ScopeSink {
            target,
            writer: Arc::new(ScopeWriter {
                default_writer: target.new_writer(),
                thread_writer: ThreadLocal::new(),
            }),
        }
    }
}

impl<C: MetricSink> MetricSink for ScopeSink<C> {
    type Metric = ScopeKey<C::Metric>;
    type Writer = ScopeWriter<C>;


    fn new_metric<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate)
            -> ScopeKey<C::Metric> {
        let pm = self.target.new_metric(m_type, name, self.sampling_rate);
        ScopeKey {
            target: pm,
        }
    }

    fn new_writer(&self) -> ScopeWriter<C> {
        self.writer.thread_writer.set(self.target.new_writer())
        // TODO drop target_writer on scope_writer drop (or something)
    }
}
