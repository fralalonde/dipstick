//! Standard stateless metric outputs.
use core::*;
use std::sync::Arc;

/// Write metric values to stdout using `println!`.
pub fn to_stdout() -> Chain<String> {
    Chain::new(
        |_kind, name, _rate| String::from(name),
        |_auto_flush| Arc::new(|cmd| if let ScopeCmd::Write(m, v) = cmd { println!("{}: {}", m, v) })
    )
}

/// Write metric values to the standard log using `info!`.
pub fn to_log() -> Chain<String> {
    Chain::new(
        |_kind, name, _rate| String::from(name),
        |_auto_flush| Arc::new(|cmd| if let ScopeCmd::Write(m, v) = cmd { info!("{}: {}", m, v) })
    )
}

/// Special sink that discards all metric values sent to it.
pub fn to_void() -> Chain<String> {
    Chain::new(
        move |_kind, name, _rate| String::from(name),
        |_auto_flush| Arc::new(|_cmd| {})
    )
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn sink_print() {
        let c = super::to_stdout();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true)(ScopeCmd::Write(&m, 33));
    }

    #[test]
    fn test_to_log() {
        let c = super::to_log();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true)(ScopeCmd::Write(&m, 33));
    }

    #[test]
    fn test_to_void() {
        let c = super::to_void();
        let m = c.define_metric(Kind::Marker, "test", 1.0);
        c.open_scope(true)(ScopeCmd::Write(&m, 33));
    }

}
