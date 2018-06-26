//! A sample application asynchronously printing metrics to stdout.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::*;
use std::io;
use std::thread;
use std::env::args;
use std::str::FromStr;

fn main() {
    let event = Proxy::default_root().marker("a");

    let bucket = Bucket::new();

    Proxy::default_root().set_target(bucket.clone());

    let args = &mut args();
    args.next();
    let tc: u8 = u8::from_str(&args.next().unwrap()).unwrap();
    for _ in 0..tc {
        let event = event.clone();
        thread::spawn(move || {
            loop {
                // report some metric values from our "application" loop
                event.mark();
            }
        });
    }
    sleep(Duration::from_secs(5));
    bucket.flush_to(&Text::output(io::stdout()).open_scope_raw(), &stats_all).unwrap();

}
