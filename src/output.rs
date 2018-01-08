//! Standard stateless metric outputs.
// TODO parameterize templates
use core::*;
use std::sync::RwLock;

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> Chain<String> {
    Chain::new(
        |_kind, name, _rate| String::from(name),
        |buffered| {
            if !buffered {
                ControlScopeFn::new(|cmd| {
                    if let ScopeCmd::Write(m, v) = cmd {
                        println!("{}: {}", m, v)
                    }
                })
            } else {
                let buf = RwLock::new(String::new());
                ControlScopeFn::new(move |cmd| {
                    let mut buf = buf.write().expect("Lock string buffer.");
                    match cmd {
                        ScopeCmd::Write(metric, value) => buf.push_str(format!("{}: {}\n", metric, value).as_ref()),
                        ScopeCmd::Flush => {
                            println!("{}", buf);
                            buf.clear();
                        }
                    }
                })
            }
        },
    )
}

/// Write metric values to the standard log using `info!`.
// TODO parameterize log level
pub fn to_log() -> Chain<String> {
    Chain::new(
        |_kind, name, _rate| String::from(name),
        |buffered| {
            if !buffered {
                ControlScopeFn::new(|cmd| {
                    if let ScopeCmd::Write(m, v) = cmd {
                        info!("{}: {}", m, v)
                    }
                })
            } else {
                let buf = RwLock::new(String::new());
                ControlScopeFn::new(move |cmd| {
                    let mut buf = buf.write().expect("Lock string buffer.");
                    match cmd {
                        ScopeCmd::Write(metric, value) => buf.push_str(format!("{}: {}\n", metric, value).as_ref()),
                        ScopeCmd::Flush => {
                            info!("{}", buf);
                            buf.clear();
                        }
                    }
                })
            }
        },
    )
}

/// Discard all metric values sent to it.
pub fn to_void() -> Chain<String> {
    Chain::new(
        move |_kind, name, _rate| String::from(name),
        |_buffered| ControlScopeFn::new(|_cmd| {}),
    )
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::to_stdout();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true).write(&m, 33);
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true).write(&m, 33);
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true).write(&m, 33);
    }

}
