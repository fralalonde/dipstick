//! Use the input metric proxy to dynamically update the metrics output & names.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::*;

fn main() {
    let root = to_proxy();
    let sub = root.add_name("sub");

    let count1 = root.counter("counter_a");

    let count2 = sub.counter("counter_b");

    loop {
        root.set_target(to_stdout().new_input());
        count1.count(1);
        count2.count(2);

        // route every metric from the root to stdout with prefix "root"
        root.set_target(to_stdout().add_name("root").new_input());
        count1.count(3);
        count2.count(4);

        // route metrics from "sub" to stdout with prefix "mutant"
        sub.set_target(to_stdout().add_name("mutant").new_input());
        count1.count(5);
        count2.count(6);

        // clear root metrics route, "sub" still appears
        root.unset_target();
        count1.count(7);
        count2.count(8);

        // now no metrics appear
        sub.unset_target();
        count1.count(9);
        count2.count(10);

        // go back to initial single unprefixed route
        root.set_target(to_stdout().new_input());
        count1.count(11);
        count2.count(12);

        sleep(Duration::from_secs(1));

        println!()
    }

}
