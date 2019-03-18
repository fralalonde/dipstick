//! Publicly exposed metric macros are defined here.

pub use lazy_static::*;

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

/// Create **Labels** from a list of key-value pairs
/// Adapted from the hashmap!() macro in the *maplit* crate.
///
/// ## Example
///
/// ```
/// #[macro_use] extern crate dipstick;
///
/// use dipstick::*;
///
/// # fn main() {
///
/// let labels = labels!{
///
///     "a" => "1",
///     "b" => "2",
/// };
/// assert_eq!(labels.lookup("a"), Some(::std::sync::Arc::new("1".into())));
/// assert_eq!(labels.lookup("b"), Some(::std::sync::Arc::new("2".into())));
/// assert_eq!(labels.lookup("c"), None);
/// # }
/// ```
#[macro_export]
macro_rules! labels {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(labels!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { labels!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = labels!(@count $($key),*);
            let mut _map: ::std::collections::HashMap<String, ::std::sync::Arc<String>> = ::std::collections::HashMap::with_capacity(_cap);
            $(
                let _ = _map.insert($key.into(), ::std::sync::Arc::new($value.into()));
            )*
            ::Labels::from(_map)
        }
    };
    () => {
        ::Labels::default()
    }
}

/// Metrics can be used from anywhere (public), does not need to declare metrics in this block.
#[macro_export]
macro_rules! metrics {
    // BRANCH NODE - public type decl
    ($(#[$attr:meta])* pub $IDENT:ident: $TYPE:ty = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: $TYPE = $e.into(); }
        metrics!{ @internal $IDENT; $TYPE; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - private typed decl
    ($(#[$attr:meta])* $IDENT:ident: $TYPE:ty = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: $TYPE = $e.into(); }
        metrics!{ @internal $IDENT; $TYPE; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - public untyped decl
    ($(#[$attr:meta])* pub $IDENT:ident = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: Proxy = $e.into(); }
        metrics!{ @internal $IDENT; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // BRANCH NODE - private untyped decl
    ($(#[$attr:meta])* $IDENT:ident = $e:expr => { $($BRANCH:tt)* } $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: Proxy = $e.into(); }
        metrics!{ @internal $IDENT; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // Identified Proxy Root
    ($e:ident => { $($BRANCH:tt)+ } $($REST:tt)*) => {
        metrics!{ @internal $e; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // Anonymous Proxy Namespace
    ($e:expr => { $($BRANCH:tt)+ } $($REST:tt)*) => {
        lazy_static! { static ref PROXY_METRICS: Proxy = $e.into(); }
        metrics!{ @internal PROXY_METRICS; Proxy; $($BRANCH)* }
        metrics!{ $($REST)* }
    };

    // LEAF NODE - public typed decl
    ($(#[$attr:meta])* pub $IDENT:ident: $TYPE:ty = $e:expr; $($REST:tt)*) => {
        metrics!{ @internal Proxy::default(); Proxy; $(#[$attr])* pub $IDENT: $TYPE = $e; }
        metrics!{ $($REST)* }
    };

    // LEAF NODE - private typed decl
    ($(#[$attr:meta])* $IDENT:ident: $TYPE:ty = $e:expr; $($REST:tt)*) => {
        metrics!{ @internal Proxy::default(); Proxy; $(#[$attr])* $IDENT: $TYPE = $e; }
        metrics!{ $($REST)* }
    };

    // END NODE
    () => ();

    // METRIC NODE - public
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* pub $IDENT:ident: $MTY:ty = $METRIC_NAME:expr; $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT: $MTY =
            $WITH.new_metric($METRIC_NAME.into(), stringify!($MTY).into()).into();
        }
        metrics!{ @internal $WITH; $TY; $($REST)* }
    };

    // METRIC NODE - private
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* $IDENT:ident: $MTY:ty = $METRIC_NAME:expr; $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT: $MTY =
            $WITH.new_metric($METRIC_NAME.into(), stringify!($MTY).into()).into();
        }
        metrics!{ @internal $WITH; $TY; $($REST)* }
    };

    // SUB BRANCH NODE - public identifier
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* pub $IDENT:ident = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* pub static ref $IDENT = $WITH.named($e); }
        metrics!( @internal $IDENT; $TY; $($BRANCH)*);
        metrics!( @internal $WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE - private identifier
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* $IDENT:ident = $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        lazy_static! { $(#[$attr])* static ref $IDENT = $WITH.named($e); }
        metrics!( @internal $IDENT; $TY; $($BRANCH)*);
        metrics!( @internal $WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE (not yet)
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* pub $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        metrics!( @internal $WITH.named($e); $TY; $($BRANCH)*);
        metrics!( @internal $WITH; $TY; $($REST)*);
    };

    // SUB BRANCH NODE (not yet)
    (@internal $WITH:expr; $TY:ty; $(#[$attr:meta])* $e:expr => { $($BRANCH:tt)*} $($REST:tt)*) => {
        metrics!( @internal $WITH.named($e); $TY; $($BRANCH)*);
        metrics!( @internal $WITH; $TY; $($REST)*);
    };

    (@internal $WITH:expr; $TYPE:ty;) => ()

}

#[cfg(test)]
mod test {
    use core::input::*;
    use core::proxy::Proxy;

    metrics! {TEST: Proxy = "test_prefix" => {
        pub M1: Marker = "failed";
        C1: Counter = "failed";
        G1: Gauge = "failed";
        T1: Timer = "failed";
    }}

    metrics!("my_app" => {
        COUNTER_A: Counter = "counter_a";
    });

    #[test]
    fn gurp() {
        COUNTER_A.count(11);
    }

    #[test]
    fn call_new_macro_defined_metrics() {
        M1.mark();
        C1.count(1);
        G1.value(1);
        T1.interval_us(1);
    }
}
