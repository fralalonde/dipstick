//! Standard stateless metric outputs.
use core::*;
use fnsink::*;

/// Write metric values to stdout using `println!`.
pub fn print() -> FnSink<String> {
    make_sink(|k, n, r| format!("{:?} {} {}", k, n, r),
                 |cmd| if let Scope::Write(m, v) = cmd {
                     println!("{}: {}", m, v)
                 })
}

/// Write metric values to the standard log using `info!`.
pub fn log<STR: AsRef<str> + 'static + Send + Sync>(prefix: STR) -> FnSink<String> {
    make_sink(move |k, n, r| format!("{}{:?} {} {}", prefix.as_ref(), k, n, r),
                 |cmd| if let Scope::Write(m, v) = cmd {
                     info!("{}: {}", m, v)
                 })
}

/// Special sink that discards all metric values sent to it.
pub fn void<STR: AsRef<str> + 'static + Send + Sync>(prefix: STR) -> FnSink<String> {
    make_sink(move |k, n, r| format!("{}{:?} {} {}", prefix.as_ref(), k, n, r),
                 |_| {})
}

mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::print();
        let m = c.new_metric(Kind::Event, "test", 1.0);
        c.new_scope()(Scope::Write(&m, 33));
    }

    #[test]
    fn log_print() {
        let c = super::log("log prefix");
        let m = c.new_metric(Kind::Event, "test", 1.0);
        c.new_scope()(Scope::Write(&m, 33));
    }

}