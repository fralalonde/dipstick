//! A sample application asynchronously printing metrics to stdout.

#[macro_use]
extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::{Proxy, Stream, Counter, InputScope, Input, Formatting, AppLabel,
               Name, Kind, LineTemplate, LineFormat, LineOp, LabelOp};

metrics!{
    COUNTER: Counter = "counter_a";
}

struct MyFormat;

impl LineFormat for MyFormat {
    fn template(&self, name: &Name, _kind: Kind) -> LineTemplate {
        vec![
            LineOp::Literal(format!("{} ", name.join("."))),
            LineOp::ValueAsText,
            LineOp::Literal(" ".into()),
            LineOp::LabelExists("abc".into(),
                vec![LabelOp::LabelKey, LabelOp::Literal(":".into()), LabelOp::LabelValue],
            ),
            LineOp::NewLine,
        ].into()
    }
}

fn main() {
    Proxy::set_default_target(Stream::stderr().formatting(MyFormat).input());
    AppLabel::set("abc", "xyz");
    loop {
        // report some metric values from our "application" loop
        COUNTER.count(11);
        sleep(Duration::from_millis(500));
    }

}
