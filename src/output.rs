//! Standard stateless metric outputs.
// TODO parameterize templates
use core::*;
use context::*;
use std::sync::RwLock;

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> MetricContext<String> {
    metric_context(
        |_kind, name, _rate| String::from(name),
        || control_scope(|cmd|
            if let ScopeCmd::Write(m, v) = cmd {
                println!("{}: {}", m, v)
            })
    )
}

/// Record metric values to stdout using `println!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
pub fn to_buffered_stdout() -> MetricContext<String> {
    metric_context(
        |_kind, name, _rate| String::from(name),
        || {
            let buf = RwLock::new(String::new());
            control_scope(move |cmd| {
                let mut buf = buf.write().expect("Locking stdout buffer");
                match cmd {
                    ScopeCmd::Write(metric, value) => {
                        buf.push_str(format!("{}: {}\n", metric, value).as_ref())
                    }
                    ScopeCmd::Flush => {
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
pub fn to_log() -> MetricContext<String> {
    metric_context(
        |_kind, name, _rate| String::from(name),
        || control_scope(|cmd|
            if let ScopeCmd::Write(m, v) = cmd {
                info!("{}: {}", m, v)
            })
    )
}

/// Record metric values to the standard log using `info!`.
/// Values are buffered until #flush is called
/// Buffered operation requires locking.
/// If thread latency is a concern you may wish to also use #with_async_queue.
// TODO parameterize log level
pub fn to_buffered_log() -> MetricContext<String> {
    metric_context(
        |_kind, name, _rate| String::from(name),
        || {
            let buf = RwLock::new(String::new());
            control_scope(move |cmd| {
                let mut buf = buf.write().expect("Locking string buffer");
                match cmd {
                    ScopeCmd::Write(metric, value) => {
                        buf.push_str(format!("{}: {}\n", metric, value).as_ref())
                    }
                    ScopeCmd::Flush => {
                        info!("{}", buf);
                        buf.clear();
                    }
                }
            })
        },
    )
}


/// Discard all metric values sent to it.
pub fn to_void() -> MetricContext<()> {
    metric_context(
        move |_kind, _name, _rate| (),
        || control_scope(|_cmd| {}),
    )
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::to_stdout().open_scope();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log().open_scope();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void().open_scope();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.write(&m, 33);
    }

}
