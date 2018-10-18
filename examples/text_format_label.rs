//! A sample application asynchronously printing metrics to stdout.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::{Stream, InputScope, Input, Formatting, AppLabel,
               Name, Kind, LineTemplate, LineFormat, LineOp, LabelOp};

struct MyFormat;

impl LineFormat for MyFormat {
    fn template(&self, name: &Name, _kind: Kind) -> LineTemplate {
        vec![
            LineOp::Literal(format!("{} ", name.join(".")).into()),
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
    let counter = Stream::stderr().formatting(MyFormat).input().counter("counter_a");
    AppLabel::set("abc", "xyz");
    loop {
        // report some metric values from our "application" loop
        counter.count(11);
        sleep(Duration::from_millis(500));
    }

}
