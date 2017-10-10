//! Standard stateless metric outputs.
use core::Scope;
use fnsink::*;

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> FnSink<String> {
    make_sink(|_, name, _| String::from(name), |cmd| {
        if let Scope::Write(m, v) = cmd {
            println!("{}: {}", m, v)
        }
    })
}

/// Write metric values to the standard log using `info!`.
pub fn to_log<STR: AsRef<str> + 'static + Send + Sync>(prefix: STR) -> FnSink<String> {
    make_sink(move |_, name, _| [prefix.as_ref(), name].concat(), |cmd| {
        if let Scope::Write(m, v) = cmd {
            info!("{}: {}", m, v)
        }
    })
}

/// Special sink that discards all metric values sent to it.
pub fn to_void() -> FnSink<String> {
    make_sink(move |_, name, _| String::from(name), |_| {})
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::to_stdout();
        let m = c.new_metric(Kind::Marker, "test", 1.0);
        c.new_scope(true)(Scope::Write(&m, 33));
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log("log prefix");
        let m = c.new_metric(Kind::Marker, "test", 1.0);
        c.new_scope(true)(Scope::Write(&m, 33));
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void();
        let m = c.new_metric(Kind::Marker, "test", 1.0);
        c.new_scope(true)(Scope::Write(&m, 33));
    }

}
