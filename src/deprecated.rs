use core::*;
use aggregate::MetricAggregator;
use scores::ScoreType;

//use async_queue::WithAsyncQueue;
//use sample::WithSamplingRate;

///// Aggregate metrics in memory.
///// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
///// Needs to be connected to a publish to be useful.
//#[deprecated(since = "0.7.0", note = "Use `MetricAggregator::new()` instead.")]
//pub fn aggregate<M, E, P>(stats_fn: E, pub_scope: P) -> MetricAggregator
//    where
//        E: Fn(Kind, Namespace, ScoreType) -> Option<(Kind, Namespace, Value)> + Send + Sync + 'static,
//        P: Into<MetricOutput<M>>,
//        M: Send + Sync + 'static + Clone,
//{
//    let agg = MetricAggregator::new();
//    agg.set_stats(stats_fn);
//    agg.set_output(pub_scope);
//    agg
//}
//
///// Enqueue collected metrics for dispatch on background thread.
//#[deprecated(since = "0.5.0", note = "Use `with_async_queue` instead.")]
//pub fn async<M, IC>(queue_size: usize, chain: IC) -> MetricOutput<M>
//    where
//        M: Clone + Send + Sync + 'static,
//        IC: Into<MetricOutput<M>>,
//{
//    let chain = chain.into();
//    chain.with_async_queue(queue_size)
//}
//
///// Perform random sampling of values according to the specified rate.
//#[deprecated(since = "0.5.0", note = "Use `with_sampling_rate` instead.")]
//pub fn sample<M, IC>(sampling_rate: Sampling, chain: IC) -> MetricOutput<M>
//    where
//        M: Clone + Send + Sync + 'static,
//        IC: Into<MetricOutput<M>>,
//{
//    let chain = chain.into();
//    chain.with_sampling_rate(sampling_rate)
//}
//
///// Wrap the metrics backend to provide an application-friendly interface.
///// Open a metric scope to share across the application.
//#[deprecated(since = "0.7.0", note = "Use into() instead")]
//pub fn app_metrics<M, AM>(scope: AM) -> MetricScope<M>
//    where
//        M: Clone + Send + Sync + 'static,
//        AM: Into<MetricScope<M>>,
//{
//    scope.into()
//}

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Metrics instead")]
pub type AppMetrics = MetricInput;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Marker instead")]
pub type AppMarker = Marker;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Counter instead")]
pub type AppCounter = Counter;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Gauge instead")]
pub type AppGauge = Gauge;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Timer instead")]
pub type AppTimer = Timer;

/// Define application-scoped metrics.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! app_metrics {
    ($type_param: ty, $metric_id: ident = ($($SCOPE: expr),+ $(,)*)) => {
        lazy_static! {
            pub static ref $metric_id: MetricScope<$type_param> = metric_scope(($($SCOPE),*));
        }
    };
    ($type_param: ty, $metric_id: ident = [$($SCOPE: expr),+ $(,)*]) => {
        lazy_static! {
            pub static ref $metric_id: MetricScope<$type_param> = metric_scope(&[$($SCOPE),*][..],);
        }
    };
    ($type_param: ty, $metric_id: ident = $SCOPE: expr) => {
        lazy_static! {
            pub static ref $metric_id: MetricScope<$type_param> = $SCOPE.into();
        }
    };
}

/// Define application-scoped markers.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! app_marker {
    (<$type_param: ty> $SCOPE: expr => { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! {
            $(pub static ref $metric_id: Marker = $SCOPE.marker( $m_exp );)*
        }
     };
}

/// Define application-scoped counters.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! app_counter {
    (<$type_param: ty> $SCOPE: expr => { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! {
            $(pub static ref $metric_id: Counter = $SCOPE.counter( $m_exp );)*
        }
    };
}

/// Define application-scoped gauges.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! app_gauge {
    (<$type_param: ty> $SCOPE: expr => { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! {
            $(pub static ref $metric_id: Gauge = $SCOPE.gauge( $m_exp );)*
        }
    };
}

/// Define application-scoped timers.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! app_timer {
    (<$type_param: ty> $SCOPE: expr => { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! {
            $(pub static ref $metric_id: Timer = $SCOPE.timer( $m_exp );)*
        }
    };
}

/////////////
// MOD SCOPE

/// Define module-scoped metrics.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! mod_metrics {
    ($type_param: ty, $metric_id: ident = ($($SCOPE: expr),+ $(,)*)) => {
        lazy_static! {
            static ref $metric_id: MetricScope<$type_param> = metric_scope(($($SCOPE),*));
        }
    };
    ($type_param: ty, $metric_id: ident = [$($SCOPE: expr),+ $(,)*]) => {
        lazy_static! { static ref $metric_id: MetricScope<$type_param> =
            metric_scope(&[$($SCOPE),*][..],); }
    };
    ($type_param: ty, $metric_id: ident = $mod_metrics: expr) => {
        lazy_static! { static ref $metric_id: MetricScope<$type_param> =
            $mod_metrics.into(); }
    };
}

/// Define module-scoped markers.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! mod_marker {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: Marker =
            $mod_metrics.marker( $m_exp );)* }
    };
}

/// Define module-scoped counters.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! mod_counter {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: Counter =
            $mod_metrics.counter( $m_exp );)* }
    };
}

/// Define module-scoped gauges.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! mod_gauge {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: Gauge =
            $mod_metrics.gauge( $m_exp );)* }
    };
    ($type_param: ty, $mod_metrics: expr, $metric_id: ident: $m_exp: expr) => {
        lazy_static! { static ref $metric_id: Gauge =
            $mod_metrics.gauge( $m_exp ); }
    }
}

/// Define module-scoped timers.
#[macro_export]
#[deprecated(since = "0.7.0", note = "Use metrics!() instead")]
macro_rules! mod_timer {
    ($type_param: ty, $mod_metrics: expr, { $($metric_id: ident: $m_exp: expr),* $(,)* } ) => {
        lazy_static! { $(static ref $metric_id: Timer =
            $mod_metrics.timer( $m_exp );)* }
    };
}

#[cfg(test)]
mod legacy_test {
    use core::*;
    use aggregate::*;
    use self_metrics::*;

    metrics!(<Aggregate> TEST_METRICS = DIPSTICK_METRICS.with_prefix("test_prefix"));

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
    fn call_old_macro_defined_metrics() {
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
