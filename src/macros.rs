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
    // typed, public, no metrics
    ($(#[$attr:meta])* <$METRIC_TYPE:ty> pub $METRIC_ID:ident = $e:expr $(;)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID: $METRIC_TYPE = $e.into(); }
    };
    // typed, public, some metrics
    ($(#[$attr:meta])* <$METRIC_TYPE:ty> pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID: $METRIC_TYPE = $e.into(); }
        __metrics_block!($METRIC_ID; $($REMAINING)*);
    };
    // typed, module, no metrics
    ($(#[$attr:meta])* <$METRIC_TYPE:ty> $METRIC_ID:ident = $e:expr $(;)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID: $METRIC_TYPE = $e.into(); }
    };
    // typed, module, some metrics
    ($(#[$attr:meta])* <$METRIC_TYPE:ty> $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID: $METRIC_TYPE = $e.into(); }
        __metrics_block!($METRIC_ID; $($REMAINING)*);
    };
    // typed, reuse predeclared
    (<$METRIC_TYPE:ty> $METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        __metrics_block!($METRIC_ID; $($REMAINING)*);
    };
    // typed, unidentified, some metrics
    (<$METRIC_TYPE:ty> $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { static ref UNIDENT_METRIC: $METRIC_TYPE = $e.into(); }
        __metrics_block!(UNIDENT_METRIC; $($REMAINING)*);
    };
    // typed, root, some metrics
    (<$METRIC_TYPE:ty> { $($REMAINING:tt)+ }) => {
        lazy_static! { static ref ROOT_METRICS: $METRIC_TYPE = ().into(); }
        __metrics_block!(ROOT_METRICS; $($REMAINING)*);
    };

    ($(#[$attr:meta])* pub $METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {$(#[$attr])* <InputProxy> pub $METRIC_ID = $e; }
    };
    ($(#[$attr:meta])* pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {$(#[$attr])* <InputProxy> pub $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($(#[$attr:meta])* $METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {$(#[$attr])* <InputProxy> $METRIC_ID = $e; }
    };
    ($(#[$attr:meta])* $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {$(#[$attr])* <InputProxy> $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($(#[$attr:meta])* $METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        metrics! {<InputProxy> $METRIC_ID => { $($REMAINING)* } }
    };
    ($e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<InputProxy> $e => { $($REMAINING)* } }
    };

}

/// Internal macro required to abstract over pub/non-pub versions of the macro
#[macro_export]
#[doc(hidden)]
macro_rules! __metrics_block {
    ($INPUT:ident;
    $(#[$attr:meta])* pub Counter $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
            Counter = $INPUT.counter($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* Counter $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
            Counter = $INPUT.counter($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* pub Marker $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
            Marker = $INPUT.marker($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* Marker $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
            Marker = $INPUT.marker($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* pub Gauge $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
            Gauge = $INPUT.gauge($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* Gauge $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
            Gauge = $INPUT.gauge($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* pub Timer $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
            Timer = $INPUT.timer($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($INPUT:ident;
    $(#[$attr:meta])* Timer $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
            Timer = $INPUT.timer($METRIC_NAME); }
        __metrics_block!($INPUT; $($REMAINING)*);
    };
    ($METRIC_ID:ident;) => ()
}


#[cfg(test)]
mod test {
    use core::*;
    use bucket::Bucket;
    use self_metrics::*;

    metrics!(<Bucket> DIPSTICK_METRICS.add_prefix("test_prefix") => {
        Marker M1: "failed";
        Marker M2: "success";
        Counter C1: "failed";
        Counter C2: "success";
        Gauge G1: "failed";
        Gauge G2: "success";
        Timer T1: "failed";
        Timer T2: "success";
    });

    #[test]
    fn call_new_macro_defined_metrics() {
        M1.mark();
        M2.mark();

        C1.count(1);
        C2.count(2);

        G1.value(1);
        G2.value(2);

        T1.interval_us(1);
        T2.interval_us(2);
    }
}
