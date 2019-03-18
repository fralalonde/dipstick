use self::LineOp::*;
use crate::core::input::InputKind;
use crate::core::name::MetricName;
use crate::core::MetricValue;

use std::io;
use std::sync::Arc;

/// Print commands are steps in the execution of output templates.
pub enum LineOp {
    /// Print a string.
    Literal(Vec<u8>),
    /// Lookup and print label value for key, if it exists.
    LabelExists(String, Vec<LabelOp>),
    /// Print metric value as text.
    ValueAsText,
    /// Print metric value, divided by the given scale, as text.
    ScaledValueAsText(f64),
    /// Print the newline character.labels.lookup(key)
    NewLine,
}

/// Print commands are steps in the execution of output templates.
pub enum LabelOp {
    /// Print a string.
    Literal(Vec<u8>),
    /// Print the label key.
    LabelKey,
    /// Print the label value.
    LabelValue,
}

/// An sequence of print commands, embodying an output strategy for a single metric.
pub struct LineTemplate {
    ops: Vec<LineOp>,
}

impl From<Vec<LineOp>> for LineTemplate {
    fn from(ops: Vec<LineOp>) -> Self {
        LineTemplate { ops }
    }
}

impl LineTemplate {
    /// Template execution applies commands in turn, writing to the output.
    pub fn print<L>(
        &self,
        output: &mut io::Write,
        value: MetricValue,
        lookup: L,
    ) -> Result<(), io::Error>
    where
        L: Fn(&str) -> Option<Arc<String>>,
    {
        for cmd in &self.ops {
            match cmd {
                Literal(src) => output.write_all(src.as_ref())?,
                ValueAsText => output.write_all(format!("{}", value).as_ref())?,
                ScaledValueAsText(scale) => {
                    let scaled = value as f64 / scale;
                    output.write_all(format!("{}", scaled).as_ref())?
                }
                NewLine => writeln!(output)?,
                LabelExists(label_key, print_label) => {
                    if let Some(label_value) = lookup(label_key.as_ref()) {
                        for label_cmd in print_label {
                            match label_cmd {
                                LabelOp::LabelValue => output.write_all(label_value.as_bytes())?,
                                LabelOp::LabelKey => output.write_all(label_key.as_bytes())?,
                                LabelOp::Literal(src) => output.write_all(src.as_ref())?,
                            }
                        }
                    }
                }
            };
        }
        Ok(())
    }
}

/// Format output config support.
pub trait Formatting {
    /// Specify formatting of output.
    fn formatting(&self, format: impl LineFormat + 'static) -> Self;
}

/// Forges metric-specific printers
pub trait LineFormat: Send + Sync {
    /// Prepare a template for output of metric values.
    fn template(&self, name: &MetricName, kind: InputKind) -> LineTemplate;
}

/// A simple metric output format of "MetricName {Value}"
#[derive(Default)]
pub struct SimpleFormat {
    // TODO make separator configurable
//    separator: String,
}

impl LineFormat for SimpleFormat {
    fn template(&self, name: &MetricName, _kind: InputKind) -> LineTemplate {
        let mut header = name.join(".");
        header.push(' ');
        LineTemplate {
            ops: vec![Literal(header.into_bytes()), ValueAsText, NewLine],
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::core::label::Labels;

    pub struct TestFormat;

    impl LineFormat for TestFormat {
        fn template(&self, name: &MetricName, kind: InputKind) -> LineTemplate {
            let mut header: String = format!("{:?}", kind);
            header.push('/');
            header.push_str(&name.join("."));
            header.push(' ');
            LineTemplate {
                ops: vec![
                    Literal(header.into()),
                    ValueAsText,
                    Literal(" ".into()),
                    ScaledValueAsText(1000.0),
                    Literal(" ".into()),
                    LabelExists(
                        "test_key".into(),
                        vec![
                            LabelOp::LabelKey,
                            LabelOp::Literal("=".into()),
                            LabelOp::LabelValue,
                        ],
                    ),
                    NewLine,
                ],
            }
        }
    }

    #[test]
    fn print_label_exists() {
        let labels: Labels = labels!("test_key" => "456");
        let format = TestFormat {};
        let mut name = MetricName::from("abc");
        name = name.prepend("xyz");
        let template = format.template(&name, InputKind::Counter);
        let mut out = vec![];
        template
            .print(&mut out, 123000, |key| labels.lookup(key))
            .unwrap();
        assert_eq!(
            "Counter/xyz.abc 123000 123 test_key=456\n",
            String::from_utf8(out).unwrap()
        );
    }

    #[test]
    fn print_label_not_exists() {
        let format = TestFormat {};
        let mut name = MetricName::from("abc");
        name = name.prepend("xyz");
        let template = format.template(&name, InputKind::Counter);
        let mut out = vec![];
        template.print(&mut out, 123000, |_key| None).unwrap();
        assert_eq!(
            "Counter/xyz.abc 123000 123 \n",
            String::from_utf8(out).unwrap()
        );
    }
}
