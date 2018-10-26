//! Print metrics to stderr with custom formatter including a label.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::{Stream, InputScope, Input, Formatting, AppLabel,
               MetricName, InputKind, LineTemplate, LineFormat, LineOp, LabelOp};

/// Generates template like "$METRIC $value $label_value["abc"]\n"
struct MyFormat;

impl LineFormat for MyFormat {
    fn template(&self, name: &MetricName, _kind: InputKind) -> LineTemplate {
        vec![
            LineOp::Literal(format!("{} ", name.join(".")).to_uppercase().into()),
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
