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
    // TYPED
    // public, no metrics
    ($(#[$attr:meta])* pub $METRIC_ID:ident = $e:expr $(;)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID: ProxyInput = $e.into(); }
    };
    // public, some metrics
    ($(#[$attr:meta])* pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID: ProxyInput = $e.into(); }
        __metrics_block!($METRIC_ID; $($REMAINING)*);
    };
    // private, no metrics
    ($(#[$attr:meta])* $METRIC_ID:ident = $e:expr $(;)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID: ProxyInput = $e.into(); }
    };
    // private, some metrics
    ($(#[$attr:meta])* $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID: ProxyInput = $e.into(); }
        __metrics_block!($METRIC_ID; $($REMAINING)*);
    };
    // no identifier, expression + some metrics
    ($e:expr => { $($REMAINING:tt)+ }) => {
        __metrics_block!($e; $($REMAINING)*);
    };
    // just metrics
    ($(#[$attr:meta])* pub $METRIC_ID:ident: $TY:ty = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        __metrics_block!(input_proxy(); $(#[$attr])* pub static ref $METRIC_ID: $TY = $METRIC_NAME; $($REMAINING)* );
    };
    // just metrics
    ($(#[$attr:meta])* $METRIC_ID:ident: $TY:ty = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        __metrics_block!(input_proxy(); $(#[$attr])* static ref $METRIC_ID: $TY = $METRIC_NAME; $($REMAINING)* );
    };
}

/// Internal macro required to abstract over pub/non-pub versions of the macro
#[macro_export]
#[doc(hidden)]
macro_rules! __metrics_block {
    // public metric
    ($INPUT:expr;
    $(#[$attr:meta])* pub $METRIC_ID:ident: $TY:ty = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! {
            $(#[$attr])* pub static ref $METRIC_ID: $TY = {
                let e: ProxyInput = $INPUT.into();
                e.new_metric($METRIC_NAME)
            }.into();
        }
        __metrics_block!($INPUT; $($REMAINING)*);
    };

    // private metric
    ($INPUT:expr;
    $(#[$attr:meta])* $METRIC_ID:ident: $TY:ty = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! {
            $(#[$attr])* static ref $METRIC_ID: $TY = {
                let e: ProxyInput = $INPUT.into();
                e.new_metric($METRIC_NAME)
            }.into();
        }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* pub $METRIC_ID:ident: Marker = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
//            Marker = { let e: ProxyInput = $INPUT.into(); e.marker($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* $METRIC_ID:ident: Marker = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
//            Marker = { let e: ProxyInput = $INPUT.into(); e.marker($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* pub $METRIC_ID:ident: Gauge = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
//            Gauge = { let e: ProxyInput = $INPUT.into(); e.gauge($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* $METRIC_ID:ident: Gauge = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
//            Gauge = { let e: ProxyInput = $INPUT.into(); e.gauge($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* pub $METRIC_ID:ident: Timer = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
//            Timer = { let e: ProxyInput = $INPUT.into(); e.timer($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
//    ($INPUT:expr;
//    $(#[$attr:meta])* $METRIC_ID:ident: Timer = $METRIC_NAME:expr; $($REMAINING:tt)*) => {
//        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
//            Timer = { let e: ProxyInput = $INPUT.into(); e.timer($METRIC_NAME) }; }
//        __metrics_block!($INPUT; $($REMAINING)*);
//    };
    ($INPUT:expr;) => ()
}


#[cfg(test)]
mod test {
    use core::*;
    use proxy::ProxyInput;

    metrics!("test_prefix" => {
        M1: Marker = "failed";
        C1: Counter = "failed";
        G1: Gauge = "failed";
        T1: Timer = "failed";
    });

    #[test]
    fn call_new_macro_defined_metrics() {
        M1.mark();
        C1.count(1);
        G1.value(1);
        T1.interval_us(1);
    }
}
