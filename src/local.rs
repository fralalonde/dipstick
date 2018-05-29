//! Standard stateless metric outputs.

// TODO parameterize templates
// TODO define backing structs that can flush() on Drop
use core::{ROOT_NS, Namespace, Sampling, Value, WriteFn, Kind, command_fn, Command, WithNamespace};
use output::{MetricOutput, metric_output};
use input::{MetricInput, DefineMetric, Flush};
use std::sync::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A HashMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone)]
pub struct StatsMap {
    namespace: Namespace,
    map: Arc<RwLock<BTreeMap<Namespace, Value>>>,
}

impl StatsMap {
    /// Create a new StatsMap.
    pub fn new() -> Self {
        StatsMap { namespace: ROOT_NS.clone(), map: Arc::new(RwLock::new(BTreeMap::new())) }
    }

    /// Get the latest published value for the named stat (if it exists).
    pub fn get(&self, key: &Namespace) -> Option<Value> {
        self.map.read().expect("StatsMap").get(key).map(|v| *v)
    }
}

impl From<StatsMap> for BTreeMap<String, Value> {
    fn from(map: StatsMap) -> Self {
        let inner = map.map.write().expect("StatsMap");
        inner.clone().into_iter()
            .map(|(key, value)| (key.join("."), value))
            .collect()
    }
}

impl DefineMetric for StatsMap {
    fn define_metric_object(&self, name: &Namespace, kind: Kind, rate: Sampling) -> WriteFn {
        let target_metric = self.define_metric(name, kind, rate);
        let write_to = self.clone();
        Arc::new(move |value| write_to.write(&target_metric, value))
    }
}

impl Flush for StatsMap {}


impl MetricInput<Namespace> for StatsMap {
    fn define_metric(&self, name: &Namespace, _kind: Kind, _rate: Sampling) -> Namespace {
        name.clone()
    }

    fn write(&self, metric: &Namespace, value: Value) {
        self.map.write().expect("StatsMap").insert(metric.clone(), value);
    }
}

impl WithNamespace for StatsMap {

    fn with_namespace(&self, namespace: &Namespace) -> Self {
        if namespace.is_empty() {
            return self.clone()
        }
        Self {
            namespace: self.namespace.with_namespace(namespace),
            map: self.map.clone(),
        }
    }

}

//pub fn to_map(map: StatsMap) -> MetricScope<Namespace> {
//    let mut map: StatsMap = map;
//    MetricScope::new(
//        ().into(),
//        Arc::new(|name, _kind, _rate| name.clone()),
//        Arc::new(|cmd|
//            if let Command::Write(m, v) = cmd {
//                map.insert(m.clone(), v);
//            },
//    )
//}

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> MetricOutput<String> {
    metric_output(
        |ns, _kind, _rate| ns.join("."),
        || {
            command_fn(|cmd| {
                if let Command::Write(m, v) = cmd {
                    println!("{}: {}", m, v)
                }
            })
        },
    )
}

/// Record metric values to stdout using `println!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
pub fn to_buffered_stdout() -> MetricOutput<String> {
    metric_output(
        |ns, _kind, _rate| ns.join("."),
        || {
            let buf = RwLock::new(String::new());
            command_fn(move |cmd| {
                let mut buf = buf.write().expect("Locking stdout buffer");
                match cmd {
                    Command::Write(metric, value) => {
                        buf.push_str(format!("{}: {}\n", metric, value).as_ref())
                    }
                    Command::Flush => {
                        println!("{}", buf);
                        buf.clear();
                    }
                }
            })
        },
    )
}

/// Write metric values to the standard log using `info!`.
// TODO parameterize log level
pub fn to_log() -> MetricOutput<String> {
    metric_output(
        |ns, _kind, _rate| ns.join("."),
        || {
            command_fn(|cmd| {
                if let Command::Write(m, v) = cmd {
                    info!("{}: {}", m, v)
                }
            })
        },
    )
}

/// Record metric values to the standard log using `info!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
// TODO parameterize log level
pub fn to_buffered_log() -> MetricOutput<String> {
    metric_output(
        |ns, _kind, _rate| ns.join("."),
        || {
            let buf = RwLock::new(String::new());
            command_fn(move |cmd| {
                let mut buf = buf.write().expect("Locking string buffer");
                match cmd {
                    Command::Write(metric, value) => {
                        buf.push_str(format!("{}: {}\n", metric, value).as_ref())
                    }
                    Command::Flush => {
                        info!("{}", buf);
                        buf.clear();
                    }
                }
            })
        },
    )
}

/// Discard all metric values sent to it.
pub fn to_void() -> MetricOutput<()> {
    metric_output(move |_ns, _kind, _rate| (), || command_fn(|_cmd| {}))
}

#[cfg(test)]
mod test {
    use core::*;
    use input::MetricInput;

    #[test]
    fn sink_print() {
        let c = super::to_stdout().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker, 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker, 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void().open_scope();
        let m = c.define_metric(&"test".into(), Kind::Marker, 1.0);
        c.write(&m, 33);
    }

}
