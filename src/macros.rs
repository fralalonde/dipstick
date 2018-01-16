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
    }}
}

/////////////
// APP SCOPE

/// Define application-scoped metrics.
#[macro_export]
macro_rules! app_metrics {
    ($type_param: ty, $metric_id: ident = $app_metrics: expr) => {
        lazy_static! { pub static ref $metric_id: AppMetrics<$type_param> = $app_metrics; }
    };
}

#[macro_export]
#[deprecated(since = "0.6.3", note = "Use `app_metrics!` instead.")]
macro_rules! app_metric {
    ($type_param: ty, $metric_id: ident = $app_metrics: expr) => {
        lazy_static! { pub static ref $metric_id: AppMetrics<$type_param> = $app_metrics; }
    };
}

/// Define application-scoped markers.
#[macro_export]
macro_rules! app_marker {
    ($type_param: ty, $app_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id: AppMarker<$type_param> = $app_metrics.marker( $metric_name );)* }
    };
}

/// Define application-scoped counters.
#[macro_export]
macro_rules! app_counter {
    ($type_param: ty, $app_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id: AppCounter<$type_param> = $app_metrics.counter( $metric_name );)* }
    };
}

/// Define application-scoped gauges.
#[macro_export]
macro_rules! app_gauge {
    ($type_param: ty, $app_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id: AppGauge<$type_param> = $app_metrics.gauge( $metric_name );)* }
    };
}

/// Define application-scoped timers.
#[macro_export]
macro_rules! app_timer {
    ($type_param: ty, $app_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(pub static ref $metric_id: AppTimer<$type_param> = $app_metrics.timer( $metric_name );)* }
    };
}


/////////////
// MOD SCOPE

/// Define module-scoped metrics.
#[macro_export]
macro_rules! mod_metrics {
    ($type_param: ty, $metric_id: ident = $mod_metrics: expr) => {
        lazy_static! { static ref $metric_id: AppMetrics<$type_param> = $mod_metrics; }
    };
}

#[macro_export]
#[deprecated(since = "0.6.3", note = "Use `mod_metrics!` instead.")]
macro_rules! mod_metric {
    ($type_param: ty, $metric_id: ident = $mod_metrics: expr) => {
        lazy_static! { static ref $metric_id: AppMetrics<$type_param> = $mod_metrics; }
    };
}

/// Define module-scoped markers.
#[macro_export]
macro_rules! mod_marker {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: AppMarker<$type_param> = $mod_metrics.marker( $metric_name );)* }
    };
}

/// Define module-scoped counters.
#[macro_export]
macro_rules! mod_counter {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: AppCounter<$type_param> = $mod_metrics.counter( $metric_name );)* }
    };
}

/// Define module-scoped gauges.
#[macro_export]
macro_rules! mod_gauge {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: AppGauge<$type_param> = $mod_metrics.gauge( $metric_name );)* }
    };
    ($type_param: ty, $mod_metrics: expr, $metric_id: ident: $metric_name: expr) => {
        lazy_static! { static ref $metric_id: AppGauge<$type_param> = $mod_metrics.gauge( $metric_name ); }
    }
}

/// Define module-scoped timers.
#[macro_export]
macro_rules! mod_timer {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $metric_name: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: AppTimer<$type_param> = $mod_metrics.timer( $metric_name );)* }
    };
}

#[cfg(test)]
mod test_app {
    use self_metrics::*;

    app_metrics!(Aggregate, TEST_METRICS = DIPSTICK_METRICS.with_prefix("test_prefix"));

    app_marker!(Aggregate, TEST_METRICS, {
        M1: "failed",
        M2: "success",
    });

    app_counter!(Aggregate, TEST_METRICS, {
        C1: "failed",
        C2: "success",
    });

    app_gauge!(Aggregate, TEST_METRICS, {
        G1: "failed",
        G2: "success",
    });

    app_timer!(Aggregate, TEST_METRICS, {
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

#[cfg(test)]
mod test_mod {
    use self_metrics::*;

    mod_metrics!(Aggregate, TEST_METRICS = DIPSTICK_METRICS.with_prefix("test_prefix"));

    mod_marker!(Aggregate, TEST_METRICS, {
        M1: "failed",
        M2: "success",
    });

    mod_counter!(Aggregate, TEST_METRICS, {
        C1: "failed",
        C2: "success",
    });

    mod_gauge!(Aggregate, TEST_METRICS, {
        G1: "failed",
        G2: "success",
    });

    mod_timer!(Aggregate, TEST_METRICS, {
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
