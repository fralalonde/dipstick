//! Metrics are printed at the end of every cycle as scope is dropped

extern crate dipstick;

use std::time::Duration;
use std::thread::sleep;
use std::io;

use dipstick::*;

fn main() {
    let input = Text::write_to(io::stdout()).with_buffering(Buffering::Unlimited);

    loop {
        println!("\n------- open scope");

        let metrics = input.input();

        metrics.marker("marker_a").mark();

        sleep(Duration::from_millis(1000));

        println!("------- close scope: ");
    }
}
