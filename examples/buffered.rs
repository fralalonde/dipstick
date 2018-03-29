//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds

extern crate dipstick;

use std::time::Duration;
use std::thread::sleep;

use dipstick::*;

fn main() {
    let config = to_buffered_stdout();

    loop {
        // add counts forever, non-stop
        println!("\n------- open scope");

        let metrics = config.open_scope();

        let counter = metrics.counter("counter_a");
        let timer = metrics.timer("timer_a");
        let gauge = metrics.gauge("gauge_a");
        let marker = metrics.marker("marker_a");

        counter.count(11);
        counter.count(12);
        counter.count(13);

        timer.interval_us(11_000_000);
        timer.interval_us(12_000_000);
        timer.interval_us(13_000_000);

        sleep(Duration::from_millis(1000));

        gauge.value(11);
        gauge.value(12);
        gauge.value(13);

        marker.mark();

        sleep(Duration::from_millis(1000));

        println!("------- close scope: ");

        // scope metrics are printed at the end of every cycle as scope is dropped
        // use scope.flush_on_drop(false) and scope.flush() to control flushing if required
    }
}
