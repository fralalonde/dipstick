//! Metrics are printed at the end of every cycle as scope is dropped

extern crate dipstick;

use std::time::Duration;
use std::thread::sleep;
use std::io;

use dipstick::*;

fn main() {
    let output = Text::output(io::stdout()).with_buffering(Buffering::Unlimited);

    loop {
        println!("\n------- open scope");

        let metrics = output.open_scope();

        metrics.marker("marker_a").mark();

        sleep(Duration::from_millis(1000));

        println!("------- close scope: ");
    }
}
