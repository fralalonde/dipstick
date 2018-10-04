//! Publicly exposed metric macros are defined here.

// TODO add #[timer("name")] custom derive

/// A convenience macro to wrap a block or an expression with a start / stop timer.
/// Elapsed time is sent to the supplied statsd client after the computation has been performed.
/// Expression result (if any) is transparently returned.
#[macro_export]
macro_rules! time {
    ($timer: expr, $body: expr) => {{
        let start_time = $timer.start();
        let value = $body;
        $timer.stop(start_time);
        value
    }};
}

/// Metrics can be used from anywhere (public), does not need to declare metrics in this block.
#[macro_export]
macro_rules! metrics {
    // BRANCH NODE - public type decl
    ($(#[$attr:meta])* pub $IDENT:ident: $TYPE:ty = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: $TYPE = $e.into(); }
        __in_context!{ $IDENT; $TYPE; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - private typed decl
    ($(#[$attr:meta])* $IDENT:ident: $TYPE:ty = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: $TYPE = $e.into(); }
        __in_context!{ $IDENT; $TYPE; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - public untyped decl
    ($(#[$attr:meta])* pub $IDENT:ident = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: Proxy = $e.into(); }
        __in_context!{ $IDENT; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - private untyped decl
    ($(#[$attr:meta])* $IDENT:ident = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: Proxy = $e.into(); }
        __in_context!{ $IDENT; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - untyped expr
    ($e:expr => { $($BRANCH:tt)+ } $($REST:tt)*) => {
        __in_context!{ $e; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // LEAF NODE - public typed decl
    ($(#[$attr:meta])* pub $IDENT:ident: $TYPE:ty = $e:expr; $($REST:tt)*) => {
        __in_context!{ Proxy::default(); Proxy; $(#[$attr])* pub $IDENT: $TYPE = $e; }
        metrics!{ $($REST)* }
    };

    // LEAF NODE - private typed decl
    ($(#[$attr:meta])* $IDENT:ident: $TYPE:ty = $e:expr; $($REST:tt)*) => {
        __in_context!{ Proxy::default(); Proxy; $(#[$attr])* $IDENT: $TYPE = $e; }
        metrics!{ $($REST)* }
    };

    // END NODE
    () => ()
}

/// Internal macro required to abstract over pub/non-pub versions of the macro
#[macro_export]
#[doc(hidden)]
macro_rules! __in_context {
    // METRIC NODE - public
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* pub $IDENT:ident: $MTY:ty = $METRIC_NAME:expr; $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: $MTY =
            $WITH.new_metric($METRIC_NAME.into(), stringify!($MTY).into()).into();
        }
        __in_context!{ $WITH; $TY; $($REST)* }
    };

    // METRIC NODE - private
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* $IDENT:ident: $MTY:ty = $METRIC_NAME:expr; $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: $MTY =
            $WITH.new_metric($METRIC_NAME.into(), stringify!($MTY).into()).into();
        }
        __in_context!{ $WITH; $TY; $($REST)* }
    };

    // SUB BRANCH NODE - public identifier
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* pub $IDENT:ident = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT = $WITH.namespace($e); }
        __in_context!($IDENT; $TY; $($BRANCH)*);
        __in_context!($WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE - private identifier
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* $IDENT:ident = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT = $WITH.namespace($e); }
        __in_context!($IDENT; $TY; $($BRANCH)*);
        __in_context!($WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE (not yet)
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* pub $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        __in_context!($WITH.namespace($e); $TY; $($BRANCH)*);
        __in_context!($WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE (not yet)
    ($WITH:expr; $TY:ty; $(#[$attr:meta])* $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        __in_context!($WITH.namespace($e); $TY; $($BRANCH)*);
        __in_context!($WITH; $TY; $($REST)*);
    };

    ($WITH:expr; $TYPE:ty;) => ()
}


#[cfg(test)]
mod test {
    use core::input::*;
    use core::proxy::Proxy;

    metrics!{TEST: Proxy = "test_prefix" => {
        M1: Marker = "failed";
        C1: Counter = "failed";
        G1: Gauge = "failed";
        T1: Timer = "failed";
    }}

    #[test]
    fn call_new_macro_defined_metrics() {
        M1.mark();
        C1.count(1);
        G1.value(1);
        T1.interval_us(1);
    }
}
