//! Publicly exposed metric macros are defined here.
//! Although `dipstick` does not have a macro-based API,
//! in some situations they can make instrumented code simpler.

// TODO add #[timer("name")] method annotation processors

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

/// AppMetrics can be used from anywhere (public), does not need to declare metrics in this block.
#[macro_export]
#[doc(hidden)]
macro_rules! app_metrics {
    // TYPED
    // typed, public, no metrics
    (<$METRIC_TYPE:ty> pub $METRIC_ID:ident = $e:expr;) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<$METRIC_TYPE> = $e.into(); }
    };
    // typed, public, some metrics
    (<$METRIC_TYPE:ty> pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<$METRIC_TYPE> = $e.into(); }
        __metrics_block!($METRIC_ID: $METRIC_TYPE; $($REMAINING)*);
    };
    // typed, module, no metrics
    (<$METRIC_TYPE:ty> $METRIC_ID:ident = $e:expr;) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<$METRIC_TYPE> = $e.into(); }
    };
    // typed, module, some metrics
    (<$METRIC_TYPE:ty> $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<$METRIC_TYPE> = $e.into(); }
        __metrics_block!($METRIC_ID: $METRIC_TYPE; $($REMAINING)*);
    };
    // typed, reuse predeclared
    (<$METRIC_TYPE:ty> $METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        __metrics_block!($METRIC_ID: $METRIC_TYPE; $($REMAINING)*);
    };
    // typed, unidentified, some metrics
    (<$METRIC_TYPE:ty> $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref UNIDENT_METRIC: AppMetrics<$METRIC_TYPE> = $e.into(); }
        __metrics_block!(UNIDENT_METRIC: $METRIC_TYPE; $($REMAINING)*);
    };
    // typed, root, some metrics
    (<$METRIC_TYPE:ty> { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref ROOT_METRICS: AppMetrics<$METRIC_TYPE> = "".into(); }
        __metrics_block!(ROOT_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };

    // DELEGATED
    // delegated, public, no metrics
    (pub $METRIC_ID:ident = $e:expr;) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<Delegate> = $e.into(); }
    };
    // delegated, public, some metrics
    (pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<Delegate> = $e.into(); }
        __metrics_block!($METRIC_ID: Delegate; $($REMAINING)*);
    };
    // delegated, module, no metrics
    ($METRIC_ID:ident = $e:expr;) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<Delegate> = $e.into(); }
    };
    // delegated, module, some metrics
    ($METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref $METRIC_ID: AppMetrics<Delegate> = $e.into(); }
        __metrics_block!($METRIC_ID: Delegate; $($REMAINING)*);
    };
    // delegated,reuse predeclared
    ($METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        __metrics_block!($METRIC_ID: Delegate; $($REMAINING)*);
    };
    // delegated, unidentified, some metrics
    ($e:expr => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref UNIDENT_METRIC: AppMetrics<Delegate> = $e.into(); }
        __metrics_block!(UNIDENT_METRIC: Delegate; $($REMAINING)*);
    };
    // delegated, root, some metrics
    ( => { $($REMAINING:tt)+ }) => {
        lazy_static! { pub static ref ROOT_METRICS: AppMetrics<Delegate> = "".into(); }
        __metrics_block!(ROOT_METRICS: Delegate; $($REMAINING)*);
    };
}

/// Internal macro required to abstract over pub/non-pub versions of the macro
#[macro_export]
#[doc(hidden)]
macro_rules! __metrics_block {
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* pub @Counter $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
        AppCounter<$METRIC_TYPE> = $APP_METRICS.counter($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* @Counter $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
        AppCounter<$METRIC_TYPE> = $APP_METRICS.counter($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* pub @Marker $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
        AppMarker<$METRIC_TYPE> = $APP_METRICS.marker($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* @Marker $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
            AppMarker<$METRIC_TYPE> = $APP_METRICS.marker($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* pub @Gauge $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
        AppGauge<$METRIC_TYPE> = $APP_METRICS.gauge($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* @Gauge $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
        AppGauge<$METRIC_TYPE> = $APP_METRICS.gauge($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* pub @Timer $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $METRIC_ID:
        AppTimer<$METRIC_TYPE> = $APP_METRICS.timer($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($APP_METRICS:ident : $METRIC_TYPE:ty;
    $(#[$attr:meta])* @Timer $METRIC_ID:ident : $METRIC_NAME:expr; $($REMAINING:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $METRIC_ID:
        AppTimer<$METRIC_TYPE> = $APP_METRICS.timer($METRIC_NAME); }
        __metrics_block!($APP_METRICS: $METRIC_TYPE; $($REMAINING)*);
    };
    ($METRIC_ID:ident : $METRIC_TYPE:ty;) => ()
}

/// Define application-scoped markers.
#[macro_export]
#[deprecated]
macro_rules! app_marker {
    (<$type_param: ty> $app_metrics: expr =>
    { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id:
        AppMarker<$type_param> = $app_metrics.marker( $metric_name );)* }
    };
}

/// Define application-scoped counters.
#[macro_export]
#[deprecated]
macro_rules! app_counter {
    (<$type_param: ty> $app_metrics: expr =>
    { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id:
        AppCounter<$type_param> = $app_metrics.counter( $metric_name );)* }
    };
}

/// Define application-scoped gauges.
#[macro_export]
#[deprecated]
macro_rules! app_gauge {
    (<$type_param: ty> $app_metrics: expr =>
    { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id:
        AppGauge<$type_param> = $app_metrics.gauge( $metric_name );)* }
    };
}

/// Define application-scoped timers.
#[macro_export]
#[deprecated]
macro_rules! app_timer {
    (<$type_param: ty> $app_metrics: expr =>
    { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id:
        AppTimer<$type_param> = $app_metrics.timer( $metric_name );)* }
    };
}

#[cfg(test)]
mod test_app {
    use self_metrics::*;

    app_metrics!(<Aggregate> TEST_METRICS = DIPSTICK_METRICS.with_prefix("test_prefix"););

    app_marker!(<Aggregate> TEST_METRICS => {
        M1: "failed",
        M2: "success",
    });

    app_counter!(<Aggregate> TEST_METRICS => {
        C1: "failed",
        C2: "success",
    });

    app_gauge!(<Aggregate> TEST_METRICS => {
        G1: "failed",
        G2: "success",
    });

    app_timer!(<Aggregate> TEST_METRICS => {
        T1: "failed",
        T2: "success",
    });

    #[test]
    fn call_macro_defined_metrics() {
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
