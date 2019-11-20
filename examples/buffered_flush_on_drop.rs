//! Metrics are printed at the end of every cycle as scope is dropped

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;

use dipstick::*;

fn main() {
    let stdout = Stream::to_stdout().buffered(Buffering::Unlimited);

    loop {
        println!("\n------- open scope");

        let metrics = stdout.locking();

        metrics.marker("marker_a").mark();

        sleep(Duration::from_millis(1000));

        println!("------- close scope: ");
    }
}
