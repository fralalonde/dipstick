//! Standard stateless metric outputs.

// TODO parameterize templates
// TODO define backing structs that can flush() on Drop
use core::*;
use output::*;
use std::sync::RwLock;

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> MetricOutput<String> {
    metric_output(
        |ns, _kind, name, _rate| ns.join(name, "."),
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
        |ns, _kind, name, _rate| ns.join(name, "."),
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
        |ns, _kind, name, _rate| ns.join(name, "."),
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
        |ns, _kind, name, _rate| ns.join(name, "."),
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
    metric_output(move |_ns, _kind, _name, _rate| (), || command_fn(|_cmd| {}))
}

#[cfg(test)]
mod test {
    use core::*;
    use scope::MetricInput;

    #[test]
    fn sink_print() {
        let c = super::to_stdout().open_scope();
        let m = c.define_metric(&ROOT_NS, Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log().open_scope();
        let m = c.define_metric(&ROOT_NS, Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void().open_scope();
        let m = c.define_metric(&ROOT_NS, Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

}
