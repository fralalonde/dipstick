//! Use the proxy to dynamically switch the metrics input & names.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use std::io;
use dipstick::{Proxy, Text, InputScope, Input, Naming};


fn main() {
    let root = Proxy::default_root();
    let sub = root.namespace("sub");

    let count1 = root.counter("counter_a");

    let count2 = sub.counter("counter_b");

    loop {
        root.set_target(Text::write_to(io::stdout()).input());
        count1.count(1);
        count2.count(2);

        // route every metric from the root to stdout with prefix "root"
        root.set_target(Text::write_to(io::stdout()).namespace("root").input());
        count1.count(3);
        count2.count(4);

        // route metrics from "sub" to stdout with prefix "mutant"
        sub.set_target(Text::write_to(io::stdout()).namespace("mutant").input());
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

        // go back to initial single un-prefixed route
        root.set_target(Text::write_to(io::stdout()).input());
        count1.count(11);
        count2.count(12);

        sleep(Duration::from_secs(1));

        println!()
    }

}
